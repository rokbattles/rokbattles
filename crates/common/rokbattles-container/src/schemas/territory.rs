//! Territory planner payloads for mesh definitions, spatial chunks, and provinces.
//!
//! Decoded collections are bounded independently of the encoded file size:
//! at most 64 MiB of vector elements and string bytes per payload. This includes
//! expanded province cells, preventing short run-length streams from requesting
//! unbounded allocations. Other collections share a 65,536-element limit to bound
//! the number of JavaScript objects created during WASM serialization. These
//! limits apply to both readers; JavaScript heap usage also depends on the engine.
//! Integers use canonical u32 varints, with zigzag encoding for i32 coordinates.

use serde::Serialize;

use crate::{Error, Value};

/// Mesh definitions: quantization scale, named vertex arrays, and triangle indices.
pub const MESH_DEFINITIONS: u16 = 401;
/// Spatial chunks: transforms, resources, landmarks, and structures.
pub const SPATIAL_CHUNK: u16 = 402;
/// Province restriction grid, encoded as runs of byte-sized province IDs.
pub const PROVINCE_GRID: u16 = 403;

/// Maximum collection and string storage decoded from one territory payload.
pub const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;

/// Maximum total collection elements, excluding the packed province cell buffer.
pub const MAX_DECODED_ELEMENTS: usize = 65_536;

/// A reusable triangle mesh in local world coordinates.
#[derive(Debug, PartialEq, Serialize)]
pub struct MeshDefinition {
    /// Mesh ID referenced by chunk instances.
    pub id: u32,
    /// Display or source name.
    pub name: String,
    /// Delta-decoded vertex coordinates, divided by the payload scale.
    pub vertices: Vec<[f64; 2]>,
    /// Triangle indices into `vertices`.
    pub indices: Vec<u32>,
}

/// A mesh placement using a two-dimensional affine transform.
#[derive(Debug, PartialEq, Serialize)]
pub struct MeshInstance {
    /// Referenced mesh definition ID.
    pub mesh: u32,
    /// Six affine coefficients in the payload's original order.
    pub affine: [f64; 6],
}

/// Resource production category.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceKind {
    /// Food production.
    Food,
    /// Wood production.
    Wood,
    /// Stone production.
    Stone,
    /// Gold production, called coin by the planner.
    Coin,
    /// Crystal production.
    Crystal,
}

/// A resource node in world coordinates.
#[derive(Debug, PartialEq, Serialize)]
pub struct ResourcePoint {
    /// Resource identifier.
    pub id: u32,
    /// Production category.
    pub kind: ResourceKind,
    /// World X coordinate.
    pub x: f64,
    /// World Y coordinate.
    pub y: f64,
}

/// Exploration landmark category.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LandmarkKind {
    /// Tribal village.
    Village,
    /// Exploration cave.
    Cave,
}

/// An exploration landmark in world coordinates.
#[derive(Debug, PartialEq, Serialize)]
pub struct MapLandmark {
    /// Landmark identifier.
    pub id: u32,
    /// Exploration category.
    pub kind: LandmarkKind,
    /// World X coordinate.
    pub x: f64,
    /// World Y coordinate.
    pub y: f64,
}

/// Static map structure category.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StructureKind {
    /// Mountain pass.
    Pass,
    /// Holy site.
    HolySite,
    /// Ancient battlefield.
    AncientBattlefield,
    /// Bastion.
    Bastion,
}

/// Area occupied by a static structure.
#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "shape", rename_all = "kebab-case")]
pub enum Collision {
    /// Square measured in world coordinates.
    WorldSquare {
        /// Distance from the center to each edge.
        #[serde(rename = "halfSize")]
        half_size: f64,
    },
    /// Square measured in territory cells.
    TerritorySquare {
        /// Radius in territory cells.
        #[serde(rename = "radiusInCells")]
        radius_in_cells: u32,
    },
}

/// A static structure with collision and territory restrictions.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapStructure {
    /// Structure identifier.
    pub id: u32,
    /// Game stronghold type.
    pub stronghold_type: u32,
    /// Structure category.
    pub kind: StructureKind,
    /// World X coordinate.
    pub x: f64,
    /// World Y coordinate.
    pub y: f64,
    /// Display label.
    pub label: String,
    /// Occupied collision area.
    pub collision: Collision,
    /// Claimed territory radius.
    pub territory_radius_in_cells: u32,
    /// Optional teleport restriction radius; absent becomes JavaScript `null`.
    pub teleport_radius_in_cells: Option<u32>,
    /// Whether an alliance can claim this structure.
    pub claimable: bool,
}

/// Contents of one spatial chunk.
#[derive(Debug, PartialEq, Serialize)]
pub struct SpatialChunk {
    /// Chunk column, not a world coordinate.
    pub x: i32,
    /// Chunk row, not a world coordinate.
    pub y: i32,
    /// Mesh placements.
    pub instances: Vec<MeshInstance>,
    /// Resource nodes.
    pub resources: Vec<ResourcePoint>,
    /// Exploration landmarks.
    pub landmarks: Vec<MapLandmark>,
    /// Static structures.
    pub structures: Vec<MapStructure>,
}

/// Expanded province restrictions in row-major order.
#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvinceGrid {
    /// World-coordinate width of a cell; must be positive.
    pub cell_size: u32,
    /// Number of columns.
    pub width: u32,
    /// Number of rows.
    pub height: u32,
    /// Server province schema identifier.
    pub server_schema: u32,
    /// Effective province schema identifier.
    pub effective_schema: u32,
    /// One province ID per cell; exported to JavaScript as `Uint8Array`.
    #[serde(serialize_with = "serialize_cells")]
    pub cells: Vec<u8>,
    /// Province IDs in which flags cannot be placed.
    pub flag_blocked: Vec<u8>,
    /// Province IDs in which fortresses cannot be placed.
    pub fortress_blocked: Vec<u8>,
}

fn serialize_cells<S: serde::Serializer>(cells: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(cells)
}

pub(crate) fn decode(schema: u16, payload: &[u8]) -> Result<Value, Error> {
    let mut reader =
        PayloadReader { bytes: payload, budget: MAX_DECODED_BYTES, elements: MAX_DECODED_ELEMENTS };
    let value = match schema {
        MESH_DEFINITIONS => Value::TerritoryMesh(reader.meshes()?),
        SPATIAL_CHUNK => Value::TerritoryChunk(reader.chunk()?),
        PROVINCE_GRID => Value::TerritoryProvince(reader.province()?),
        id => return Err(Error::UnknownSchema(id)),
    };
    if !reader.bytes.is_empty() {
        return Err(invalid("trailing territory payload bytes"));
    }
    Ok(value)
}

fn invalid(message: &str) -> Error {
    Error::InvalidPayload(message.into())
}

struct PayloadReader<'a> {
    bytes: &'a [u8],
    budget: usize,
    elements: usize,
}

impl PayloadReader<'_> {
    fn byte(&mut self) -> Result<u8, Error> {
        let (&byte, rest) =
            self.bytes.split_first().ok_or_else(|| invalid("truncated territory payload"))?;
        self.bytes = rest;
        Ok(byte)
    }

    fn uint(&mut self) -> Result<u32, Error> {
        let mut value = 0_u32;
        for shift in (0..35).step_by(7) {
            let byte = self.byte()?;
            if shift == 28 && byte > 15 {
                return Err(invalid("territory varint exceeds u32"));
            }
            value |= u32::from(byte & 127) << shift;
            if byte & 128 == 0 {
                if shift != 0 && byte == 0 {
                    return Err(invalid("noncanonical territory varint"));
                }
                return Ok(value);
            }
        }
        Err(invalid("territory varint is too long"))
    }

    fn sint(&mut self) -> Result<i32, Error> {
        let value = self.uint()?;
        let magnitude =
            i32::try_from(value >> 1).map_err(|_error| invalid("signed varint overflow"))?;
        Ok(if value & 1 == 0 { magnitude } else { -magnitude - 1 })
    }

    fn positive(&mut self) -> Result<u32, Error> {
        let value = self.uint()?;
        if value == 0 {
            return Err(invalid("zero territory scale or dimension"));
        }
        Ok(value)
    }

    fn charge(&mut self, bytes: usize) -> Result<(), Error> {
        self.budget = self.budget.checked_sub(bytes).ok_or(Error::PayloadTooLarge)?;
        Ok(())
    }

    fn vector<T>(&mut self, count: usize) -> Result<Vec<T>, Error> {
        self.elements = self.elements.checked_sub(count).ok_or(Error::PayloadTooLarge)?;
        self.storage(count)
    }

    fn storage<T>(&mut self, count: usize) -> Result<Vec<T>, Error> {
        self.charge(count.checked_mul(size_of::<T>()).ok_or(Error::PayloadTooLarge)?)?;
        let mut output = Vec::new();
        output.try_reserve_exact(count).map_err(|_error| Error::PayloadTooLarge)?;
        Ok(output)
    }

    fn count(&mut self, minimum_bytes: usize) -> Result<usize, Error> {
        let count = usize::try_from(self.uint()?).map_err(|_error| Error::PayloadTooLarge)?;
        // Check the wire bound before allocating, even when the output budget fits.
        if count > self.bytes.len() / minimum_bytes {
            return Err(invalid("territory count exceeds remaining payload"));
        }
        Ok(count)
    }

    fn text(&mut self) -> Result<String, Error> {
        let length = self.count(1)?;
        let (text, rest) = self.bytes.split_at(length);
        let text = std::str::from_utf8(text)?;
        self.charge(length)?;
        let mut output = String::new();
        output.try_reserve_exact(length).map_err(|_error| Error::PayloadTooLarge)?;
        output.push_str(text);
        self.bytes = rest;
        Ok(output)
    }

    fn meshes(&mut self) -> Result<Vec<MeshDefinition>, Error> {
        let scale = f64::from(self.positive()?);
        let count = self.count(4)?;
        let mut definitions = self.vector(count)?;
        for _ in 0..count {
            let id = self.uint()?;
            let name = self.text()?;
            let count = self.count(2)?;
            let mut vertices = self.vector(count)?;
            let (mut x, mut y) = (0_i32, 0_i32);
            for _ in 0..count {
                x = x
                    .checked_add(self.sint()?)
                    .ok_or_else(|| invalid("vertex coordinate overflow"))?;
                y = y
                    .checked_add(self.sint()?)
                    .ok_or_else(|| invalid("vertex coordinate overflow"))?;
                vertices.push([f64::from(x) / scale, f64::from(y) / scale]);
            }
            let count = self.count(1)?;
            if count % 3 != 0 {
                return Err(invalid("incomplete mesh triangle"));
            }
            let mut indices = self.vector(count)?;
            for _ in 0..count {
                let index = self.uint()?;
                if u64::from(index) >= vertices.len() as u64 {
                    return Err(invalid("mesh index outside vertices"));
                }
                indices.push(index);
            }
            definitions.push(MeshDefinition { id, name, vertices, indices });
        }
        Ok(definitions)
    }

    fn chunk(&mut self) -> Result<SpatialChunk, Error> {
        let scale = f64::from(self.positive()?);
        let x = self.sint()?;
        let y = self.sint()?;
        let count = self.count(7)?;
        let mut instances = self.vector(count)?;
        for _ in 0..count {
            let mesh = self.uint()?;
            let mut affine = [0.0; 6];
            for coefficient in &mut affine {
                *coefficient = f64::from(self.sint()?) / scale;
            }
            instances.push(MeshInstance { mesh, affine });
        }
        let count = self.count(4)?;
        let mut resources = self.vector(count)?;
        for _ in 0..count {
            let id = self.uint()?;
            let kind = match self.byte()? {
                0 | 1 => ResourceKind::Food,
                2 => ResourceKind::Wood,
                3 => ResourceKind::Stone,
                4 => ResourceKind::Coin,
                5 => ResourceKind::Crystal,
                _ => return Err(invalid("unknown resource kind")),
            };
            resources.push(ResourcePoint {
                id,
                kind,
                x: f64::from(self.sint()?) / scale,
                y: f64::from(self.sint()?) / scale,
            });
        }
        let count = self.count(4)?;
        let mut landmarks = self.vector(count)?;
        for _ in 0..count {
            let id = self.uint()?;
            let kind = match self.byte()? {
                0 | 1 => LandmarkKind::Village,
                2 => LandmarkKind::Cave,
                _ => return Err(invalid("unknown landmark kind")),
            };
            landmarks.push(MapLandmark {
                id,
                kind,
                x: f64::from(self.sint()?) / scale,
                y: f64::from(self.sint()?) / scale,
            });
        }
        let count = self.count(11)?;
        let mut structures = self.vector(count)?;
        for _ in 0..count {
            let id = self.uint()?;
            let stronghold_type = self.uint()?;
            let kind = match self.byte()? {
                0 | 1 => StructureKind::Pass,
                2 => StructureKind::HolySite,
                3 => StructureKind::AncientBattlefield,
                4 => StructureKind::Bastion,
                _ => return Err(invalid("unknown structure kind")),
            };
            let x = f64::from(self.sint()?) / scale;
            let y = f64::from(self.sint()?) / scale;
            let shape = self.byte()?;
            let amount = self.uint()?;
            let collision = match shape {
                1 => Collision::WorldSquare { half_size: f64::from(amount) / scale },
                2 => Collision::TerritorySquare { radius_in_cells: amount },
                _ => return Err(invalid("unknown collision shape")),
            };
            let territory_radius_in_cells = self.uint()?;
            let teleport_radius_in_cells = self.uint()?.checked_sub(1);
            let flags = self.byte()?;
            if flags & !1 != 0 {
                return Err(invalid("unknown structure flags"));
            }
            let label = self.text()?;
            structures.push(MapStructure {
                id,
                stronghold_type,
                kind,
                x,
                y,
                label,
                collision,
                territory_radius_in_cells,
                teleport_radius_in_cells,
                claimable: flags & 1 != 0,
            });
        }
        Ok(SpatialChunk { x, y, instances, resources, landmarks, structures })
    }

    fn blocked(&mut self) -> Result<Vec<u8>, Error> {
        let count = self.count(1)?;
        let mut ids = self.vector(count)?;
        for _ in 0..count {
            ids.push(
                u8::try_from(self.uint()?).map_err(|_error| invalid("province ID exceeds u8"))?,
            );
        }
        Ok(ids)
    }

    fn province(&mut self) -> Result<ProvinceGrid, Error> {
        let server_schema = self.uint()?;
        let effective_schema = self.uint()?;
        let cell_size = self.positive()?;
        let width = self.positive()?;
        let height = self.positive()?;
        let flag_blocked = self.blocked()?;
        let fortress_blocked = self.blocked()?;
        let count = self.count(2)?;
        let length = usize::try_from(u64::from(width) * u64::from(height))
            .map_err(|_error| Error::PayloadTooLarge)?;
        // Cells become one Uint8Array, so only the byte budget applies to this buffer.
        let mut cells = self.storage(length)?;
        for _ in 0..count {
            let run = usize::try_from(self.uint()?).map_err(|_error| Error::PayloadTooLarge)?;
            let id = self.byte()?;
            let end = cells
                .len()
                .checked_add(run)
                .filter(|&end| run != 0 && end <= length)
                .ok_or_else(|| invalid("province run exceeds grid"))?;
            cells.resize(end, id);
        }
        if cells.len() != length {
            return Err(invalid("incomplete province grid"));
        }
        Ok(ProvinceGrid {
            cell_size,
            width,
            height,
            server_schema,
            effective_schema,
            cells,
            flag_blocked,
            fortress_blocked,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MESH: &[u8] = &[10, 1, 7, 2, 0xc3, 0xa9, 3, 0, 0, 40, 0, 39, 40, 3, 0, 1, 2];
    const CHUNK: &[u8] = &[
        10, 3, 4, 1, 7, 20, 0, 0, 20, 60, 79, 1, 9, 4, 50, 99, 1, 10, 2, 100, 119, 2, 11, 12, 1,
        20, 39, 1, 30, 4, 0, 1, 2, b'a', b'b', 13, 14, 4, 0, 0, 2, 2, 3, 5, 0, 0,
    ];
    const PROVINCE: &[u8] = &[5, 6, 10, 3, 2, 2, 1, 2, 1, 3, 2, 2, 1, 4, 3];

    fn reader(bytes: &[u8]) -> PayloadReader<'_> {
        PayloadReader { bytes, budget: MAX_DECODED_BYTES, elements: MAX_DECODED_ELEMENTS }
    }

    fn put_uint(bytes: &mut Vec<u8>, mut value: u32) {
        while value >= 128 {
            bytes.push((value as u8 & 127) | 128);
            value >>= 7;
        }
        bytes.push(value as u8);
    }

    #[test]
    fn decodes_triangle_coordinates_and_utf8_name() {
        let Value::TerritoryMesh(meshes) = decode(MESH_DEFINITIONS, MESH).expect("mesh") else {
            panic!("wrong schema");
        };
        assert_eq!(
            meshes,
            vec![MeshDefinition {
                id: 7,
                name: "é".into(),
                vertices: vec![[0.0, 0.0], [2.0, 0.0], [0.0, 2.0]],
                indices: vec![0, 1, 2],
            }]
        );
    }

    #[test]
    fn decodes_chunk_records_and_collision_variants() {
        let Value::TerritoryChunk(chunk) = decode(SPATIAL_CHUNK, CHUNK).expect("chunk") else {
            panic!("wrong schema");
        };
        assert_eq!((chunk.x, chunk.y), (-2, 2));
        assert_eq!(
            chunk.instances,
            vec![MeshInstance { mesh: 7, affine: [1.0, 0.0, 0.0, 1.0, 3.0, -4.0] }]
        );
        assert_eq!(
            chunk.resources,
            vec![ResourcePoint { id: 9, kind: ResourceKind::Coin, x: 2.5, y: -5.0 }]
        );
        assert_eq!(
            chunk.landmarks,
            vec![MapLandmark { id: 10, kind: LandmarkKind::Cave, x: 5.0, y: -6.0 }]
        );
        assert_eq!(
            chunk.structures,
            vec![
                MapStructure {
                    id: 11,
                    stronghold_type: 12,
                    kind: StructureKind::Pass,
                    x: 1.0,
                    y: -2.0,
                    label: "ab".into(),
                    collision: Collision::WorldSquare { half_size: 3.0 },
                    territory_radius_in_cells: 4,
                    teleport_radius_in_cells: None,
                    claimable: true
                },
                MapStructure {
                    id: 13,
                    stronghold_type: 14,
                    kind: StructureKind::Bastion,
                    x: 0.0,
                    y: 0.0,
                    label: "".into(),
                    collision: Collision::TerritorySquare { radius_in_cells: 2 },
                    territory_radius_in_cells: 3,
                    teleport_radius_in_cells: Some(4),
                    claimable: false
                },
            ]
        );
    }

    #[test]
    fn expands_province_runs_in_row_order() {
        let Value::TerritoryProvince(grid) = decode(PROVINCE_GRID, PROVINCE).expect("grid") else {
            panic!("wrong schema");
        };
        assert_eq!(
            grid,
            ProvinceGrid {
                cell_size: 10,
                width: 3,
                height: 2,
                server_schema: 5,
                effective_schema: 6,
                cells: vec![1, 1, 3, 3, 3, 3],
                flag_blocked: vec![1, 2],
                fortress_blocked: vec![3],
            }
        );
    }

    #[test]
    fn rejects_truncation_and_trailing_bytes_in_every_schema() {
        for (id, payload) in
            [(MESH_DEFINITIONS, MESH), (SPATIAL_CHUNK, CHUNK), (PROVINCE_GRID, PROVINCE)]
        {
            for length in 0..payload.len() {
                assert!(decode(id, &payload[..length]).is_err(), "schema {id}, length {length}");
            }
            let mut trailing = payload.to_vec();
            trailing.push(0);
            decode(id, &trailing).unwrap_err();
        }
    }

    #[test]
    fn rejects_varint_overflow_and_noncanonical_encodings() {
        assert_eq!(reader(&[255, 255, 255, 255, 15]).uint().expect("max"), u32::MAX);
        assert_eq!(reader(&[255, 255, 255, 255, 15]).sint().expect("min"), i32::MIN);
        assert_eq!(reader(&[254, 255, 255, 255, 15]).sint().expect("max"), i32::MAX);
        for bytes in
            [&[128][..], &[128, 0], &[255, 255, 255, 255, 16], &[128, 128, 128, 128, 128, 1]]
        {
            reader(bytes).uint().unwrap_err();
        }
    }

    #[test]
    fn rejects_invalid_mesh_fields() {
        for (offset, value) in [(0, 0), (4, 255), (16, 3), (13, 2)] {
            let mut payload = MESH.to_vec();
            payload[offset] = value;
            decode(MESH_DEFINITIONS, &payload).unwrap_err();
        }
        // Two cumulative x deltas cannot exceed the signed coordinate range.
        let mut payload = vec![1, 1, 0, 0, 2];
        put_uint(&mut payload, u32::MAX - 1);
        payload.extend_from_slice(&[0, 2, 0, 0]);
        decode(MESH_DEFINITIONS, &payload).unwrap_err();
    }

    #[test]
    fn rejects_unknown_record_tags_and_flags() {
        for offset in [14, 19, 25, 28, 32] {
            let mut payload = CHUNK.to_vec();
            payload[offset] = 255;
            assert!(decode(SPATIAL_CHUNK, &payload).is_err(), "offset {offset}");
        }
    }

    #[test]
    fn rejects_invalid_runs_dimensions_and_large_allocations() {
        for (offset, value) in [(2, 0), (3, 0), (4, 0), (11, 0), (11, 7), (13, 3)] {
            let mut payload = PROVINCE.to_vec();
            payload[offset] = value;
            decode(PROVINCE_GRID, &payload).unwrap_err();
        }
        let mut huge_grid = vec![0, 0, 1];
        put_uint(&mut huge_grid, u32::MAX);
        put_uint(&mut huge_grid, u32::MAX);
        huge_grid.extend_from_slice(&[0, 0, 1, 1, 1]);
        assert!(matches!(decode(PROVINCE_GRID, &huge_grid), Err(Error::PayloadTooLarge)));
        let mut huge_count = vec![1];
        put_uint(&mut huge_count, u32::MAX);
        decode(MESH_DEFINITIONS, &huge_count).unwrap_err();
        let mut bounded = PayloadReader { bytes: MESH, budget: 1, elements: MAX_DECODED_ELEMENTS };
        assert!(matches!(bounded.meshes(), Err(Error::PayloadTooLarge)));
    }

    #[test]
    fn collection_limit_bounds_compact_records_and_accumulates() {
        let mut payload = vec![1];
        put_uint(&mut payload, 65_537);
        payload.resize(payload.len() + 65_537 * 4, 0);
        assert!(matches!(decode(MESH_DEFINITIONS, &payload), Err(Error::PayloadTooLarge)));
        let mut bounded = reader(&[]);
        bounded.elements = 3;
        assert_eq!(bounded.vector::<u8>(2).expect("first collection").capacity(), 2);
        assert!(matches!(bounded.vector::<u8>(2), Err(Error::PayloadTooLarge)));
        assert_eq!(bounded.storage::<u8>(4).expect("packed cells").capacity(), 4);
    }

    #[test]
    fn arbitrary_short_inputs_do_not_panic() {
        let mut state = 42_u32;
        for length in 0..128 {
            let bytes: Vec<u8> = (0..length)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    state as u8
                })
                .collect();
            for id in [MESH_DEFINITIONS, SPATIAL_CHUNK, PROVINCE_GRID] {
                drop(decode(id, &bytes));
            }
        }
    }

    #[cfg(feature = "write")]
    #[test]
    fn reader_dispatches_the_registered_schemas() {
        for (id, payload) in
            [(MESH_DEFINITIONS, MESH), (SPATIAL_CHUNK, CHUNK), (PROVINCE_GRID, PROVINCE)]
        {
            let expected = decode(id, payload).expect("payload");
            let mut file = crate::write_envelope(id, payload, 42).expect("write");
            let decoded =
                crate::Reader::default().decode_expected(&mut file, Some(id)).expect("read");
            assert_eq!(decoded.value, expected);
        }
    }
}
