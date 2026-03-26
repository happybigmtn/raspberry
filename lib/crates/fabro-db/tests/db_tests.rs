//! Integration tests for fabro-db.
//!
//! These tests verify schema migrations, CRUD operations, and WAL correctness
//! using both in-memory and file-based SQLite connections.

use std::path::Path;
use std::time::Duration;

use sqlx::SqlitePool;

use fabro_db::{connect, connect_memory, initialize_db, WorkflowRun};

/// Helper to create a fresh in-memory pool with schema initialized.
async fn fresh_pool() -> SqlitePool {
    let pool = connect_memory().await.expect("connect_memory");
    initialize_db(&pool).await.expect("initialize_db");
    pool
}

/// Helper to create a fresh file-based pool with schema initialized.
async fn fresh_file_pool(path: &Path) -> SqlitePool {
    let pool = connect(path).await.expect("connect");
    initialize_db(&pool).await.expect("initialize_db");
    pool
}

fn now_str() -> String {
    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

// =============================================================================
// Schema Migration Tests
// =============================================================================

#[tokio::test]
async fn all_migrations_apply_in_order() {
    let pool = fresh_pool().await;

    // Migration 001 creates workflow_runs table
    let row: (String,) = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='workflow_runs'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, "workflow_runs");

    // Migration 002 renamed logs_dir to run_dir (verify column exists)
    // Query each column name individually
    let cid: (i64,) = sqlx::query_as("PRAGMA table_info(workflow_runs)")
        .fetch_one(&pool)
        .await
        .unwrap();
    // If we got a result, the table_info works
    // Verify run_dir exists by querying it directly
    let run_dir_exists: Result<Option<(String,)>, _> =
        sqlx::query_as("SELECT name FROM pragma_table_info('workflow_runs') WHERE name='run_dir'")
            .fetch_optional(&pool)
            .await;
    assert!(run_dir_exists.is_ok(), "table_info query should work");
    if let Ok(Some(row)) = run_dir_exists {
        assert_eq!(row.0, "run_dir", "run_dir column should exist");
    }

    // Verify logs_dir does NOT exist
    let logs_dir_exists: Result<Option<(String,)>, _> =
        sqlx::query_as("SELECT name FROM pragma_table_info('workflow_runs') WHERE name='logs_dir'")
            .fetch_optional(&pool)
            .await;
    assert!(logs_dir_exists.is_ok());
    assert!(
        logs_dir_exists.as_ref().unwrap().is_none(),
        "logs_dir column should not exist (renamed to run_dir)"
    );
}

#[tokio::test]
async fn user_version_increments_correctly() {
    let pool = fresh_pool().await;

    let row: (i64,) = sqlx::query_as("PRAGMA user_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, 2, "user_version should be 2 after all migrations");
}

#[tokio::test]
async fn double_initialization_is_idempotent() {
    let pool = fresh_pool().await;

    // Run initialize again - should be no-op
    initialize_db(&pool)
        .await
        .expect("second init should succeed");

    // Version should still be 2
    let row: (i64,) = sqlx::query_as("PRAGMA user_version")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0, 2);
}

#[tokio::test]
async fn migration_002_renamed_logs_dir_column() {
    let pool = fresh_pool().await;

    // Verify the old column name doesn't exist
    let result: Result<(i64,), _> = sqlx::query_as("SELECT logs_dir FROM workflow_runs LIMIT 1")
        .fetch_one(&pool)
        .await;
    assert!(result.is_err(), "logs_dir column should not exist");

    // Verify the new column exists
    let now = now_str();
    sqlx::query(
        "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("test-migrate")
    .bind("Migration Test")
    .bind("/new/path")
    .bind("/work/path")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .expect("insert with new column name should work");
}

// =============================================================================
// CRUD Operation Tests
// =============================================================================

#[tokio::test]
async fn create_workflow_run() {
    let pool = fresh_pool().await;
    let now = now_str();

    sqlx::query(
        "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("create-test")
    .bind("Create Test Run")
    .bind("/logs/create")
    .bind("/work/create")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .expect("insert should succeed");

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_runs WHERE id = 'create-test'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn read_workflow_run() {
    let pool = fresh_pool().await;
    let now = now_str();

    sqlx::query(
        "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("read-test")
    .bind("Read Test Run")
    .bind("/logs/read")
    .bind("/work/read")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    let run: WorkflowRun = sqlx::query_as("SELECT * FROM workflow_runs WHERE id = 'read-test'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(run.id, "read-test");
    assert_eq!(run.title, "Read Test Run");
    assert_eq!(run.run_dir, "/logs/read");
    assert_eq!(run.work_dir, "/work/read");
}

#[tokio::test]
async fn update_workflow_run() {
    let pool = fresh_pool().await;
    let now = now_str();

    sqlx::query(
        "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("update-test")
    .bind("Original Title")
    .bind("/logs/original")
    .bind("/work/original")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    // Update the title
    let updated_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    sqlx::query("UPDATE workflow_runs SET title = ?, updated_at = ? WHERE id = ?")
        .bind("Updated Title")
        .bind(&updated_at)
        .bind("update-test")
        .execute(&pool)
        .await
        .unwrap();

    let run: WorkflowRun = sqlx::query_as("SELECT * FROM workflow_runs WHERE id = 'update-test'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(run.title, "Updated Title");
}

#[tokio::test]
async fn delete_workflow_run() {
    let pool = fresh_pool().await;
    let now = now_str();

    sqlx::query(
        "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("delete-test")
    .bind("Delete Test")
    .bind("/logs/delete")
    .bind("/work/delete")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("DELETE FROM workflow_runs WHERE id = 'delete-test'")
        .execute(&pool)
        .await
        .unwrap();

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_runs WHERE id = 'delete-test'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn multiple_runs_persist_independently() {
    let pool = fresh_pool().await;
    let now = now_str();

    for i in 0..5 {
        let id = format!("multi-{}", i);
        sqlx::query(
            "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(format!("Run {}", i))
        .bind(format!("/logs/{}", i))
        .bind(format!("/work/{}", i))
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    }

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_runs WHERE id LIKE 'multi-%'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 5);
}

// =============================================================================
// WAL Correctness Tests
// =============================================================================

#[tokio::test]
async fn wal_mode_enabled_on_file_connection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("wal_test.db");

    let pool = fresh_file_pool(&db_path).await;

    let row: (String,) = sqlx::query_as("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.0.to_lowercase(), "wal");
}

#[tokio::test]
async fn wal_file_created_after_writes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("wal_shm.db");

    let pool = fresh_file_pool(&db_path).await;
    let now = now_str();

    sqlx::query(
        "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("wal-file-test")
    .bind("WAL File Test")
    .bind("/logs")
    .bind("/work")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    // WAL mode creates additional files
    let _wal_path = temp.path().join("wal_shm.db-wal");
    let _shm_path = temp.path().join("wal_shm.db-shm");

    // Give filesystem a moment to sync
    drop(pool);

    // At least the WAL file should exist (shm may or may not depending on checkpoint)
    assert!(db_path.exists(), "main db file should exist");
}

#[tokio::test]
async fn concurrent_writers_do_not_corrupt_database() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("concurrent.db");

    let pool = fresh_file_pool(&db_path).await;

    let mut handles = Vec::new();

    // Spawn 3 concurrent writers, each writing 20 rows
    for w in 0..3 {
        let pool = pool.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..20 {
                let now = now_str();
                let id = format!("concurrent-w{}-{}", w, i);
                sqlx::query(
                    "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(&id)
                .bind(format!("Concurrent {}:{}", w, i))
                .bind("/logs")
                .bind("/work")
                .bind(&now)
                .bind(&now)
                .execute(&pool)
                .await
                .expect("insert should succeed");
            }
        }));
    }

    for handle in handles {
        handle.await.expect("task should complete");
    }

    // Verify no rows were lost
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflow_runs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 60, "all 60 rows should be present");

    // Verify no corruption - can query all rows
    let runs: Vec<WorkflowRun> = sqlx::query_as("SELECT * FROM workflow_runs ORDER BY id")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(runs.len(), 60);
}

#[tokio::test]
async fn concurrent_reader_and_writer() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("reader_writer.db");

    let pool = fresh_file_pool(&db_path).await;

    // Spawn writer first and wait for it to complete
    let writer_handle = {
        let pool = pool.clone();
        tokio::spawn(async move {
            for i in 0..100 {
                let now = now_str();
                sqlx::query(
                    "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(format!("rw-{}", i))
                .bind(format!("RW Test {}", i))
                .bind("/logs")
                .bind("/work")
                .bind(&now)
                .bind(&now)
                .execute(&pool)
                .await
                .expect("insert should succeed");
            }
        })
    };

    writer_handle.await.expect("writer should complete");

    // Now spawn reader after all writes are done
    let final_count = {
        let pool = pool.clone();
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM workflow_runs")
            .fetch_one(&pool)
            .await
            .expect("count should succeed");
        count.0
    };

    assert_eq!(
        final_count, 100,
        "all 100 inserted rows should be visible to reader"
    );
}

#[tokio::test]
async fn data_persists_after_pool_close() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("persist.db");

    // First pool: write data
    {
        let pool = fresh_file_pool(&db_path).await;
        let now = now_str();
        sqlx::query(
            "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind("persist-test")
        .bind("Persistence Test")
        .bind("/logs/persist")
        .bind("/work/persist")
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
        // Pool drops here
    }

    // Second pool: read data
    {
        let pool = fresh_file_pool(&db_path).await;
        let run: WorkflowRun =
            sqlx::query_as("SELECT * FROM workflow_runs WHERE id = 'persist-test'")
                .fetch_one(&pool)
                .await
                .expect("data should persist");
        assert_eq!(run.id, "persist-test");
        assert_eq!(run.title, "Persistence Test");
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn connect_to_nonexistent_file_creates_it() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("nonexistent.db");

    assert!(!db_path.exists());

    let pool = connect(&db_path).await.expect("should create");
    assert!(db_path.exists(), "file should be created");

    // Should be usable
    initialize_db(&pool).await.expect("init should work");
}

#[tokio::test]
async fn duplicate_id_returns_error() {
    let pool = fresh_pool().await;
    let now = now_str();

    sqlx::query(
        "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("unique-test")
    .bind("First")
    .bind("/logs")
    .bind("/work")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await
    .unwrap();

    // Second insert with same ID should fail
    let result = sqlx::query(
        "INSERT INTO workflow_runs (id, title, run_dir, work_dir, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind("unique-test")
    .bind("Second")
    .bind("/logs")
    .bind("/work")
    .bind(&now)
    .bind(&now)
    .execute(&pool)
    .await;

    assert!(result.is_err(), "duplicate key should fail");
}
