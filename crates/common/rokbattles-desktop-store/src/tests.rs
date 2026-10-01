use super::*;

fn state() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().canonicalize().expect("canonical tempdir").join("state");
    (temp, path)
}

async fn root(store: &Store, directory: &Path) -> Root {
    store.set_roots(&[directory.join("mailcache")]).await.expect("roots");
    store.roots().await.expect("roots").remove(0)
}

#[tokio::test]
async fn pending_and_settings_survive_restart_without_json_import() {
    let (_temp, directory) = state();
    let store = Store::open(&directory).await.expect("open");
    let root = root(&store, &directory).await;
    let path = root.path.join("Persistent.Mail.123");
    let sig = FileSig { size: 12, modified_ms: 4 };
    assert!(store.observe(root.id, &path, sig).await.expect("observe"));
    assert!(!store.observe(root.id, &path, sig).await.expect("same"));
    assert!(!store.settings().await.expect("settings").capture_opt_in);
    store.set_paused(true).await.expect("pause");
    store.close().await;
    let store = Store::open(&directory).await.expect("reopen");
    assert!(store.settings().await.expect("settings").paused);
    let item = store.next_pending(0).await.expect("pending").expect("item");
    assert_eq!(item.path, path);
    assert_eq!(item.sig, sig);
    assert!(!format!("{item:?}").contains("Persistent.Mail"));
    store.finish(&item, true).await.expect("finish");
    assert!(store.next_pending(0).await.expect("pending").is_none());
    assert_eq!(store.status().await.expect("status").completed, 1);
    store.close().await;
}

#[tokio::test]
async fn stale_completion_never_removes_changed_file_and_retry_is_bounded() {
    let (_temp, directory) = state();
    let store = Store::open(&directory).await.expect("open");
    let root = root(&store, &directory).await;
    let path = root.path.join("Persistent.Mail.123");
    store.observe(root.id, &path, FileSig { size: 12, modified_ms: 4 }).await.expect("observe");
    let old = store.next_pending(0).await.expect("pending").expect("item");
    store.observe(root.id, &path, FileSig { size: 13, modified_ms: 5 }).await.expect("changed");
    store.finish(&old, true).await.expect("stale finish");
    let item = store.next_pending(0).await.expect("pending").expect("changed item");
    assert_eq!(item.sig.size, 13);
    store.retry(&item, 100).await.expect("retry");
    assert!(store.next_pending(99).await.expect("pending").is_none());
    assert_eq!(store.next_pending(100).await.expect("pending").expect("item").attempts, 1);
    store.finish(&item, false).await.expect("unsupported");
    assert_eq!(store.status().await.expect("status").rejected, 1);
    store.close().await;
}

#[tokio::test]
async fn roots_cascade_and_reset_restore_exact_usage() {
    let (_temp, directory) = state();
    let store = Store::open(&directory).await.expect("open");
    let root = root(&store, &directory).await;
    store
        .observe(root.id, &root.path.join("Persistent.Mail.1"), FileSig { size: 1, modified_ms: 1 })
        .await
        .expect("observe");
    store.clear_history().await.expect("clear");
    assert_eq!(store.status().await.expect("status").pending, 0);
    store
        .observe(root.id, &root.path.join("Persistent.Mail.2"), FileSig { size: 1, modified_ms: 1 })
        .await
        .expect("observe");
    store.set_roots(&[]).await.expect("remove");
    let usage: (i64, i64) = sqlx::query_as("SELECT rows,path_bytes FROM usage WHERE id=1")
        .fetch_one(&store.pool)
        .await
        .expect("usage");
    assert_eq!(usage, (0, 0));
    store.request_reprocess().await.expect("request reset");
    assert_eq!(store.settings().await.expect("settings").reset_epoch, 1);
    store.close().await;
    Store::open(&directory).await.expect("reopen").close().await;
}

#[tokio::test]
async fn only_one_agent_owns_private_lock() {
    let (_temp, directory) = state();
    let store = Store::open(&directory).await.expect("open");
    let lease = store.acquire_agent().expect("lease").expect("first");
    assert!(store.acquire_agent().expect("second").is_none());
    drop(lease);
    assert!(store.acquire_agent().expect("released").is_some());
    store.close().await;
}

#[tokio::test]
async fn invalid_inputs_and_extra_schema_are_rejected() {
    let (_temp, directory) = state();
    let store = Store::open(&directory).await.expect("open");
    assert!(store.set_roots(&[PathBuf::from("relative")]).await.is_err());
    let root = root(&store, &directory).await;
    store
        .observe(root.id, &directory.join("outside"), FileSig { size: 1, modified_ms: 1 })
        .await
        .expect_err("outside root");
    store
        .observe(
            root.id,
            &root.path.join("large"),
            FileSig { size: MAX_MAIL_BYTES + 1, modified_ms: 1 },
        )
        .await
        .expect_err("oversize");
    sqlx::query("CREATE TABLE injected (data BLOB)")
        .execute(&store.pool)
        .await
        .expect("tamper fixture");
    store.close().await;
    assert!(Store::open(&directory).await.is_err());
}

#[tokio::test]
async fn aggregate_path_quota_rolls_back_without_losing_prior_pending() {
    let (_temp, directory) = state();
    let store = Store::open(&directory).await.expect("open");
    let root = root(&store, &directory).await;
    let mut tx = store.pool.begin().await.expect("transaction");
    let suffix = "x".repeat(3900 - root.path.to_str().expect("path").len());
    let mut accepted = 0;
    for index in 0..3000 {
        let path = root.path.join(format!("{index:04}{suffix}"));
        let result = sqlx::query("INSERT INTO files VALUES(?,?,1,1,0,0,0)")
            .bind(path.to_str().expect("text"))
            .bind(root.id)
            .execute(&mut *tx)
            .await;
        if result.is_err() {
            break;
        }
        accepted += 1;
    }
    assert!(accepted > 2000 && accepted < 3000);
    tx.commit().await.expect("commit accepted rows");
    let (rows, bytes): (i64, i64) = sqlx::query_as("SELECT rows,path_bytes FROM usage WHERE id=1")
        .fetch_one(&store.pool)
        .await
        .expect("usage");
    assert_eq!(rows, accepted);
    assert!(bytes <= MAX_PATH_TOTAL as i64);
    let next = root.path.join(format!("next{suffix}"));
    assert!(
        !store
            .observe(root.id, &next, FileSig { size: 1, modified_ms: 2 })
            .await
            .expect("bounded full queue")
    );
    let oldest = store.next_pending(0).await.expect("pending").expect("oldest");
    store.finish(&oldest, true).await.expect("completed history");
    assert!(
        store
            .observe(root.id, &next, FileSig { size: 1, modified_ms: 2 })
            .await
            .expect("reclaim completed history")
    );
    assert_eq!(
        store.status().await.expect("status").pending,
        u64::try_from(accepted).expect("nonnegative count")
    );
    store.clear_history().await.expect("bulk clear");
    assert!(sidecar_size(&directory, "-wal").expect("wal") < CHECKPOINT_BYTES);
    store.close().await;
    Store::open(&directory).await.expect("reopen after bulk write").close().await;
}

#[test]
fn hot_journal_and_recovery_files_are_checked_before_open() {
    let (_temp, directory) = state();
    rokbattles_desktop_paths::directory(&directory).expect("directory");
    let journal =
        rokbattles_desktop_paths::file(&directory, "worker.sqlite3-journal").expect("journal");
    journal.set_len(MAX_RECOVERY_BYTES + 1).expect("sparse oversize fixture");
    assert!(check_files(&directory).is_err());
}

#[tokio::test]
async fn missing_file_cleanup_preserves_newer_signature() {
    let (_temp, directory) = state();
    let store = Store::open(&directory).await.expect("open");
    let root = root(&store, &directory).await;
    let path = root.path.join("Persistent.Mail.123");
    store.observe(root.id, &path, FileSig { size: 1, modified_ms: 1 }).await.expect("observe");
    let stale = store.next_pending(0).await.expect("pending").expect("item");
    store.observe(root.id, &path, FileSig { size: 2, modified_ms: 2 }).await.expect("changed");
    store.forget_missing(&stale).await.expect("stale removal");
    let current = store.next_pending(0).await.expect("pending").expect("changed item");
    assert_eq!(current.sig.size, 2);
    store.forget_missing(&current).await.expect("missing cleanup");
    assert!(store.next_pending(0).await.expect("pending").is_none());
    store.close().await;
}
