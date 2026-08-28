//! Compact, chunked Combat Lab precomputation.

mod catalog;
mod loadout;
mod model;
mod pipeline;

use std::{
    collections::{BTreeMap, HashMap},
    mem,
    time::Instant,
};

use futures::{StreamExt, stream};
use mongodb::{
    Collection,
    bson::{Bson, DateTime, Document, doc},
    options::Hint,
};
use rokbattles_api::db::ReportsStore;
use rokbattles_bson::{bson_to_f64, bson_to_i64};

use self::{
    catalog::Catalogs,
    loadout::{accumulate_snapshot, map_projected_loadout, pack_month},
    model::{MonthLoadouts, PairingKey, PairingRoot, PerformancePoint, RawTotals, range_cutoffs},
    pipeline::{loadout_pipeline, performance_pipeline},
};
use crate::{error::JobsError, precompute_cmdr_pairings::legendary_commander_ids};

const PERFORMANCE_KIND: i64 = 1;
const LOADOUT_KIND: i64 = 2;
const ROOT_KIND: i64 = 0;
const BULK_WRITE_BATCH_SIZE: usize = 500;
const SAFE_BSON_BYTES: usize = 12 * 1024 * 1024;
const PERFORMANCE_CHUNK_MS: i64 = 32 * model::DAY_MS;
const LOADOUT_CHUNK_MS: i64 = 126 * model::DAY_MS;
const PARTITION_CONCURRENCY: usize = 12;
const DAILY_LOADOUT_DAYS: i64 = 8;

/// Counts and timings from one compact Combat Lab refresh.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CommanderPairingsV2PrecomputeStats {
    pub legendary_commanders: usize,
    pub pairings: usize,
    pub performance_points: usize,
    pub loadout_snapshots: usize,
    pub documents_written: usize,
    pub max_document_bytes: usize,
    pub performance_seconds: u64,
    pub loadout_seconds: u64,
    pub total_seconds: u64,
}

/// Refresh compact Combat Lab data while reusing DRASTC from the existing pairing collection.
pub async fn precompute_commander_pairings_v2_data(
    reports_store: &ReportsStore,
) -> Result<CommanderPairingsV2PrecomputeStats, JobsError> {
    let started = Instant::now();
    let generation = DateTime::now();
    let now_ms = generation.timestamp_millis();
    let cutoffs = range_cutoffs(now_ms);
    let daily_loadout_cutoff = now_ms - DAILY_LOADOUT_DAYS * model::DAY_MS;
    let legendary_ids = legendary_commander_ids()?;
    let catalogs = Catalogs::load()?;
    let output = reports_store.precomputed_commander_pairings_v2_collection();
    let mut roots = read_stored_drastc(reports_store).await?;
    let performance_partitions = time_partitions(cutoffs[0], now_ms, PERFORMANCE_CHUNK_MS);
    let mut documents_written = 0_usize;
    let mut max_document_bytes = 0_usize;

    let performance_started = Instant::now();
    let mut performance_points = 0_usize;
    let mut performance_runs = stream::iter(performance_partitions)
        .map(|(start_ms, end_ms)| {
            read_performance_partition(
                reports_store.battle_collection(),
                output,
                &legendary_ids,
                start_ms,
                end_ms,
                generation,
                &cutoffs,
            )
        })
        .buffer_unordered(PARTITION_CONCURRENCY);
    while let Some(result) = performance_runs.next().await {
        let result = result?;
        merge_roots(&mut roots, result.roots);
        performance_points += result.rows;
        documents_written += result.documents_written;
        max_document_bytes = max_document_bytes.max(result.max_document_bytes);
    }
    let performance_seconds = performance_started.elapsed().as_secs();

    let loadout_started = Instant::now();
    let mut loadout_snapshots = 0_usize;
    let mut governor_last_seen = HashMap::<(PairingKey, i64, i64), i64>::new();
    let loadout_partitions = time_partitions(cutoffs[0], now_ms, LOADOUT_CHUNK_MS);
    let loadout_context = LoadoutPartitionContext {
        source: reports_store.battle_collection(),
        output,
        legendary_ids: &legendary_ids,
        daily_cutoff_ms: daily_loadout_cutoff,
        generation,
        catalogs: &catalogs,
    };
    let mut loadout_runs = stream::iter(loadout_partitions)
        .map(|(start_ms, end_ms)| read_loadout_partition(&loadout_context, start_ms, end_ms))
        .buffer_unordered(PARTITION_CONCURRENCY);
    while let Some(result) = loadout_runs.next().await {
        let result = result?;
        merge_governors(&mut governor_last_seen, result.governors);
        loadout_snapshots += result.rows;
        documents_written += result.documents_written;
        max_document_bytes = max_document_bytes.max(result.max_document_bytes);
    }
    finalize_all_governors(&mut roots, &cutoffs, governor_last_seen);
    let loadout_seconds = loadout_started.elapsed().as_secs();

    let pairings = roots.len();
    let mut writer = BulkWriter::new(output);
    for (key, root) in roots {
        writer.push(root_document(key, root, generation)?).await?;
    }
    writer.flush().await?;
    documents_written += writer.documents_written;
    max_document_bytes = max_document_bytes.max(writer.max_document_bytes);

    // New roots are published only after all of their generation's chunks exist. Readers can
    // select the newest root and use its generation while this removes the previous snapshot.
    output.delete_many(doc! { "g": { "$ne": generation } }).await?;

    Ok(CommanderPairingsV2PrecomputeStats {
        legendary_commanders: legendary_ids.len(),
        pairings,
        performance_points,
        loadout_snapshots,
        documents_written,
        max_document_bytes,
        performance_seconds,
        loadout_seconds,
        total_seconds: started.elapsed().as_secs(),
    })
}

async fn read_stored_drastc(
    reports_store: &ReportsStore,
) -> Result<BTreeMap<PairingKey, PairingRoot>, JobsError> {
    let mut cursor = reports_store
        .precomputed_commander_pairings_collection()
        .find(doc! {})
        .projection(doc! {
            "_id": 0,
            "primary_commander_id": 1,
            "secondary_commander_id": 1,
            "drastc": 1,
        })
        .await?;
    let mut roots = BTreeMap::<PairingKey, PairingRoot>::new();
    while let Some(next) = cursor.next().await {
        let document = next?;
        let Some(key) = pairing_key(&document, "primary_commander_id", "secondary_commander_id")
        else {
            continue;
        };
        roots.entry(key).or_default().drastc = pack_drastc(&document);
    }
    Ok(roots)
}

fn time_partitions(first_ms: i64, now_ms: i64, width_ms: i64) -> Vec<(i64, i64)> {
    let end = now_ms.saturating_add(1);
    let mut partitions = Vec::new();
    let mut start = first_ms;
    while start < end {
        let aligned_next =
            start.saturating_sub(start.rem_euclid(width_ms)).saturating_add(width_ms);
        let next = aligned_next.min(end);
        partitions.push((start, next));
        start = next;
    }
    partitions
}

fn merge_roots(
    destination: &mut BTreeMap<PairingKey, PairingRoot>,
    sources: BTreeMap<PairingKey, PairingRoot>,
) {
    for (pairing, source) in sources {
        let destination = destination.entry(pairing).or_default();
        for (key, totals) in source.summaries {
            destination.summaries.entry(key).or_default().accumulate(totals);
        }
    }
}

fn merge_governors(
    destination: &mut HashMap<(PairingKey, i64, i64), i64>,
    sources: HashMap<(PairingKey, i64, i64), i64>,
) {
    for (key, day) in sources {
        destination.entry(key).and_modify(|current| *current = (*current).max(day)).or_insert(day);
    }
}

struct PerformancePartition {
    roots: BTreeMap<PairingKey, PairingRoot>,
    rows: usize,
    documents_written: usize,
    max_document_bytes: usize,
}

async fn read_performance_partition(
    source: &Collection<Document>,
    output: &Collection<Document>,
    legendary_ids: &[i64],
    start_ms: i64,
    end_ms: i64,
    generation: DateTime,
    cutoffs: &[i64; 4],
) -> Result<PerformancePartition, JobsError> {
    let mut cursor = source
        .aggregate(performance_pipeline(legendary_ids, start_ms, end_ms))
        .allow_disk_use(true)
        .batch_size(1_000)
        .hint(Hint::Keys(doc! {
            "metadata.mail_time": -1,
            "metadata.kvk": 1,
            "opponents.player_id": 1,
        }))
        .await?;
    let mut writer = BulkWriter::new(output);
    let mut roots = BTreeMap::<PairingKey, PairingRoot>::new();
    let mut points = 0_usize;

    while let Some(next) = cursor.next().await {
        let document = next?;
        let pairing = pairing_key(&document, "p", "s").ok_or_else(|| {
            JobsError::InvalidCombatLabData("performance chunk is missing commander IDs".to_owned())
        })?;
        let month = document_i64(&document, "m").ok_or_else(|| {
            JobsError::InvalidCombatLabData("performance chunk is missing its month".to_owned())
        })?;
        let records = document.get_array("v").cloned().map_err(|_| {
            JobsError::InvalidCombatLabData("performance chunk has no records".to_owned())
        })?;
        for record in &records {
            let point = map_performance_record(pairing, month, record)?;
            roots.entry(pairing).or_default().accumulate(point, cutoffs);
            points += 1;
        }
        write_chunk_records(&mut writer, PERFORMANCE_KIND, pairing, month, generation, 0, records)
            .await?;
    }
    writer.flush().await?;
    Ok(PerformancePartition {
        roots,
        rows: points,
        documents_written: writer.documents_written,
        max_document_bytes: writer.max_document_bytes,
    })
}

struct LoadoutPartition {
    governors: HashMap<(PairingKey, i64, i64), i64>,
    rows: usize,
    documents_written: usize,
    max_document_bytes: usize,
}

struct LoadoutPartitionContext<'a> {
    source: &'a Collection<Document>,
    output: &'a Collection<Document>,
    legendary_ids: &'a [i64],
    daily_cutoff_ms: i64,
    generation: DateTime,
    catalogs: &'a Catalogs,
}

async fn read_loadout_partition(
    context: &LoadoutPartitionContext<'_>,
    start_ms: i64,
    end_ms: i64,
) -> Result<LoadoutPartition, JobsError> {
    let mut cursor = context
        .source
        .aggregate(loadout_pipeline(
            context.legendary_ids,
            start_ms,
            end_ms,
            context.daily_cutoff_ms,
        ))
        .allow_disk_use(true)
        .batch_size(1_000)
        .hint(Hint::Keys(doc! {
            "metadata.mail_time": -1,
            "metadata.kvk": 1,
            "opponents.player_id": 1,
        }))
        .await?;
    let mut writer = BulkWriter::new(context.output);
    let mut governor_last_seen = HashMap::<(PairingKey, i64, i64), i64>::new();
    let mut snapshots = 0_usize;

    while let Some(next) = cursor.next().await {
        let document = next?;
        let pairing = pairing_key(&document, "p", "s").ok_or_else(|| {
            JobsError::InvalidCombatLabData("loadout chunk is missing commander IDs".to_owned())
        })?;
        let month_start = document_i64(&document, "m").ok_or_else(|| {
            JobsError::InvalidCombatLabData("loadout chunk is missing its range".to_owned())
        })?;
        let day = document_i64(&document, "d").ok_or_else(|| {
            JobsError::InvalidCombatLabData("loadout chunk is missing its day".to_owned())
        })?;
        let scenario = document_i64(&document, "c").ok_or_else(|| {
            JobsError::InvalidCombatLabData("loadout chunk is missing its scenario".to_owned())
        })?;
        let records = document.get_array("v").map_err(|_| {
            JobsError::InvalidCombatLabData("loadout chunk has no records".to_owned())
        })?;
        let mut month = MonthLoadouts { pairing, month: month_start, ..MonthLoadouts::default() };
        for record in records {
            let snapshot = map_projected_loadout(record).map_err(|error| {
                JobsError::InvalidCombatLabData(format!("invalid loadout aggregation row: {error}"))
            })?;
            governor_last_seen
                .entry((pairing, snapshot.c, snapshot.u))
                .and_modify(|day| *day = (*day).max(snapshot.d))
                .or_insert(snapshot.d);
            accumulate_snapshot(&mut month, &snapshot, context.catalogs);
            snapshots += 1;
        }
        let part = ((day - month_start).div_euclid(model::DAY_MS) * 5 + scenario) * 1_000;
        write_loadout_month(&mut writer, month, context.generation, part).await?;
    }
    writer.flush().await?;
    Ok(LoadoutPartition {
        governors: governor_last_seen,
        rows: snapshots,
        documents_written: writer.documents_written,
        max_document_bytes: writer.max_document_bytes,
    })
}

async fn write_loadout_month(
    writer: &mut BulkWriter<'_>,
    month: MonthLoadouts,
    generation: DateTime,
    first_part: i64,
) -> Result<(), JobsError> {
    let pairing = month.pairing;
    let month_start = month.month;
    write_chunk_records(
        writer,
        LOADOUT_KIND,
        pairing,
        month_start,
        generation,
        first_part,
        pack_month(month),
    )
    .await
}

fn finalize_all_governors(
    roots: &mut BTreeMap<PairingKey, PairingRoot>,
    cutoffs: &[i64; 4],
    last_seen: HashMap<(PairingKey, i64, i64), i64>,
) {
    for ((pairing, scenario, _player), day) in last_seen {
        let root = roots.entry(pairing).or_default();
        for (range, cutoff) in cutoffs.iter().enumerate() {
            if day >= *cutoff {
                *root.governors.entry((range as i64, scenario)).or_default() += 1;
            }
        }
    }
}

async fn write_chunk_records(
    writer: &mut BulkWriter<'_>,
    kind: i64,
    pairing: PairingKey,
    month: i64,
    generation: DateTime,
    first_part: i64,
    records: Vec<Bson>,
) -> Result<(), JobsError> {
    let mut part = first_part;
    let mut current = Vec::new();

    for record in records {
        current.push(record);
        let candidate = chunk_document(kind, pairing, month, generation, part, current.clone());
        if encoded_size(&candidate)? > SAFE_BSON_BYTES {
            let record = current.pop().expect("record was just pushed");
            if current.is_empty() {
                return Err(JobsError::InvalidCombatLabData(
                    "one packed Combat Lab record exceeds the safe BSON limit".to_owned(),
                ));
            }
            writer
                .push(chunk_document(
                    kind,
                    pairing,
                    month,
                    generation,
                    part,
                    mem::take(&mut current),
                ))
                .await?;
            part += 1;
            current.push(record);
        }
    }

    if !current.is_empty() {
        writer.push(chunk_document(kind, pairing, month, generation, part, current)).await?;
    }
    Ok(())
}

fn chunk_document(
    kind: i64,
    pairing: PairingKey,
    month: i64,
    generation: DateTime,
    part: i64,
    records: Vec<Bson>,
) -> Document {
    doc! {
        "k": kind,
        "p": pairing.primary,
        "s": pairing.secondary,
        "g": generation,
        "m": month,
        "q": part,
        "v": records,
    }
}

fn root_document(
    pairing: PairingKey,
    root: PairingRoot,
    generation: DateTime,
) -> Result<Document, JobsError> {
    let mut summaries = Vec::new();
    for ((range, scenario), totals) in root.summaries {
        summaries.push(Bson::Array(vec![
            range.into(),
            scenario.into(),
            totals.battles.into(),
            root.governors.get(&(range, scenario)).copied().unwrap_or_default().into(),
            totals.kill_points_gained.into(),
            totals.kill_points_lost.into(),
            totals.severely_wounded_inflicted.into(),
            totals.severely_wounded_taken.into(),
            totals.battle_duration_ms.into(),
            totals.rate_duration_ms.into(),
            totals.damage.into(),
            totals.healing.into(),
        ]));
    }
    let mut document = doc! {
        "k": ROOT_KIND,
        "p": pairing.primary,
        "s": pairing.secondary,
        "g": generation,
        "q": 0_i64,
        "r": summaries,
    };
    if let Some(drastc) = root.drastc {
        document.insert("d", drastc);
    }
    ensure_safe_size(&document)?;
    Ok(document)
}

fn map_performance_record(
    pairing: PairingKey,
    month: i64,
    value: &Bson,
) -> Result<PerformancePoint, JobsError> {
    let values = value.as_array().ok_or_else(|| {
        JobsError::InvalidCombatLabData("performance record is not an array".to_owned())
    })?;
    let required = |index: usize| {
        values.get(index).and_then(bson_to_i64).ok_or_else(|| {
            JobsError::InvalidCombatLabData(format!(
                "performance record is missing tuple index {index}"
            ))
        })
    };
    Ok(PerformancePoint {
        pairing,
        month,
        day: required(0)?,
        scenario: required(1)?,
        totals: RawTotals {
            battles: required(2)?,
            kill_points_gained: required(3)?,
            kill_points_lost: required(4)?,
            severely_wounded_inflicted: required(5)?,
            severely_wounded_taken: required(6)?,
            battle_duration_ms: required(7)?,
            rate_duration_ms: required(8)?,
            damage: required(9)?,
            healing: required(10)?,
        },
    })
}

fn pack_drastc(document: &Document) -> Option<Bson> {
    let drastc = document.get_document("drastc").ok()?;
    let confidence = drastc.get_document("confidence").ok()?;
    let breakdown = drastc.get_document("breakdown").ok()?;
    let categories = ["damage", "rage", "assist", "sustainability", "trade", "consistency"]
        .into_iter()
        .map(|key| {
            let category = breakdown.get_document(key).ok()?;
            Some(Bson::Array(vec![
                number(category, "value")?.into(),
                number(category, "p10")?.into(),
                number(category, "p90")?.into(),
                number(category, "score")?.into(),
            ]))
        })
        .collect::<Option<Vec<_>>>()?;

    Some(Bson::Array(vec![
        document_i64(drastc, "samples")?.into(),
        number(drastc, "overall")?.into(),
        number(confidence, "score")?.into(),
        document_i64(confidence, "unique_governors")?.into(),
        number(confidence, "effective_governors")?.into(),
        Bson::Array(categories),
    ]))
}

fn pairing_key(document: &Document, primary: &str, secondary: &str) -> Option<PairingKey> {
    Some(PairingKey {
        primary: document.get(primary).and_then(bson_to_i64)?,
        secondary: document.get(secondary).and_then(bson_to_i64)?,
    })
}

fn document_i64(document: &Document, key: &str) -> Option<i64> {
    document.get(key).and_then(bson_to_i64)
}

fn number(document: &Document, key: &str) -> Option<f64> {
    document.get(key).and_then(bson_to_f64)
}

fn encoded_size(document: &Document) -> Result<usize, JobsError> {
    Ok(mongodb::bson::to_vec(document)?.len())
}

fn ensure_safe_size(document: &Document) -> Result<usize, JobsError> {
    let size = encoded_size(document)?;
    if size > SAFE_BSON_BYTES {
        return Err(JobsError::InvalidCombatLabData(format!(
            "packed document is {size} bytes; safe limit is {SAFE_BSON_BYTES}"
        )));
    }
    Ok(size)
}

struct BulkWriter<'a> {
    output: &'a Collection<Document>,
    models: Vec<mongodb::options::ReplaceOneModel>,
    documents_written: usize,
    max_document_bytes: usize,
}

impl<'a> BulkWriter<'a> {
    fn new(output: &'a Collection<Document>) -> Self {
        Self {
            output,
            models: Vec::with_capacity(BULK_WRITE_BATCH_SIZE),
            documents_written: 0,
            max_document_bytes: 0,
        }
    }

    async fn push(&mut self, document: Document) -> Result<(), JobsError> {
        let size = ensure_safe_size(&document)?;
        self.max_document_bytes = self.max_document_bytes.max(size);
        let mut selector = doc! {
            "k": document_i64(&document, "k").unwrap_or_default(),
            "p": document_i64(&document, "p").unwrap_or_default(),
            "s": document_i64(&document, "s").unwrap_or_default(),
            "g": document.get_datetime("g").copied().unwrap_or(DateTime::from_millis(0)),
            "q": document_i64(&document, "q").unwrap_or_default(),
        };
        if let Some(month) = document.get("m").and_then(bson_to_i64) {
            selector.insert("m", month);
        } else {
            selector.insert("m", Bson::Null);
        }
        let mut model = self.output.replace_one_model(selector, &document)?;
        model.upsert = Some(true);
        self.models.push(model);
        if self.models.len() >= BULK_WRITE_BATCH_SIZE {
            self.flush().await?;
        }
        Ok(())
    }

    async fn flush(&mut self) -> Result<(), JobsError> {
        if self.models.is_empty() {
            return Ok(());
        }
        let batch = mem::replace(&mut self.models, Vec::with_capacity(BULK_WRITE_BATCH_SIZE));
        let count = batch.len();
        self.output.client().bulk_write(batch).ordered(false).await?;
        self.documents_written += count;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_root_omits_unavailable_drastc_and_long_field_names() {
        let document = root_document(
            PairingKey { primary: 1, secondary: 2 },
            PairingRoot::default(),
            DateTime::from_millis(10),
        )
        .expect("root");

        assert!(!document.contains_key("d"));
        assert!(!format!("{document:?}").contains("schemaVersion"));
        assert!(!format!("{document:?}").contains("primaryCommanderName"));
    }

    #[test]
    fn chunk_documents_remain_below_the_safety_margin() {
        let records = (0..50_000)
            .map(|value| Bson::Array(vec![value.into(), value.into(), value.into()]))
            .collect();
        let document = chunk_document(
            PERFORMANCE_KIND,
            PairingKey { primary: 1, secondary: 2 },
            0,
            DateTime::from_millis(0),
            0,
            records,
        );

        assert!(ensure_safe_size(&document).is_ok());
    }

    #[test]
    fn time_partitions_snap_to_aligned_boundaries_after_the_partial_first_range() {
        let partitions = time_partitions(15, 45, 10);

        assert_eq!(partitions, vec![(15, 20), (20, 30), (30, 40), (40, 46)]);
    }
}
