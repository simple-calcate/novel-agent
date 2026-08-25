//! 迁移升级路径：手工构造只应用了 0001+0002 的旧库，再通过
//! `Repository::open` 增量升级到 0003，数据不丢、新列/新表可用。

use novel_domain::ProjectId;
use novel_storage::Repository;
use rusqlite::{params, Connection};

const MIGRATION_0001: &str = include_str!("../migrations/0001_init.sql");
const MIGRATION_0002: &str = include_str!("../migrations/0002_app_settings.sql");

fn build_legacy_db(path: &std::path::Path) {
    let connection = Connection::open(path).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .unwrap();
    connection.execute_batch(MIGRATION_0001).unwrap();
    connection.execute_batch(MIGRATION_0002).unwrap();
    // 旧库没有 schema_migrations 概念，写入已应用版本
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();
    for version in [1i64, 2] {
        connection
            .execute(
                "INSERT INTO schema_migrations(version) VALUES (?1)",
                [version],
            )
            .unwrap();
    }
    // 旧库写入一条业务数据，升级后必须还在
    let legacy_project = uuid::Uuid::new_v4().to_string();
    let legacy_job = uuid::Uuid::new_v4().to_string();
    connection
        .execute(
            "INSERT INTO projects(id, title, created_at, updated_at) VALUES (?1, '旧作品', '2026-01-01', '2026-01-01')",
            params![legacy_project],
        )
        .unwrap();
    // 旧库已有一条 pending 任务
    connection
        .execute(
            "INSERT INTO jobs(id, project_id, operation, payload_json, priority, status,
                idempotency_key, depends_on_json, attempts, max_attempts, run_at,
                created_at, updated_at)
             VALUES (?1, ?2, 'document.save', '{}', 0, 'pending',
                'legacy-key', '[]', 0, 3, '2026-01-01T00:00:00Z',
                '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
            params![legacy_job, legacy_project],
        )
        .unwrap();
}

#[test]
fn legacy_database_upgrades_to_latest_schema() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("legacy.sqlite3");
    build_legacy_db(&db_path);

    // 打开即触发增量迁移
    let repository = Repository::open(&db_path).unwrap();

    // 旧数据仍在
    assert!(repository.get_setting("nothing").unwrap().is_none());
    let jobs = repository.list_jobs(10).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].operation, "document.save");
    assert_eq!(jobs[0].idempotency_key, "legacy-key");

    // 0003 新能力可用：冷却表 + 任务结果列
    let rule = novel_domain::WorkflowRule {
        id: Default::default(),
        project_id: ProjectId::new(),
        name: "升级后规则".into(),
        enabled: true,
        trigger: novel_domain::WorkflowTrigger {
            event_type: "editor.idle".into(),
        },
        conditions: vec![],
        actions: vec![],
        priority: 0,
        cooldown_ms: 1000,
    };
    assert!(!repository
        .workflow_in_cooldown(&rule, "editor.idle", chrono::Utc::now())
        .unwrap());
    repository
        .record_workflow_fired(&rule.id, "editor.idle", chrono::Utc::now())
        .unwrap();
    assert!(repository
        .workflow_in_cooldown(&rule, "editor.idle", chrono::Utc::now())
        .unwrap());

    // 升级后的旧任务能正常领取并把结果写进新列 result_json
    let claimed = repository
        .claim_next_job(chrono::Utc::now(), chrono::Duration::minutes(10))
        .unwrap()
        .expect("旧任务应可领取");
    repository
        .complete_job(
            &claimed.id,
            &serde_json::json!({"saved": true}),
            chrono::Utc::now(),
        )
        .unwrap();

    // 重复打开（幂等迁移）不报错
    drop(repository);
    let repository = Repository::open(&db_path).unwrap();
    assert_eq!(
        repository.list_jobs(10).unwrap()[0].status,
        novel_domain::JobStatus::Succeeded
    );
}
