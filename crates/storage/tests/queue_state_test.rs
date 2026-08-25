use chrono::{Duration, Utc};
use novel_domain::{
    Job, JobId, JobStatus, ProjectId, WorkflowCondition, WorkflowRule, WorkflowTrigger,
};
use novel_storage::Repository;

fn pending_job(key: &str, depends_on: Vec<JobId>) -> Job {
    let now = Utc::now();
    Job {
        id: JobId::new(),
        project_id: ProjectId::new(),
        workflow_id: None,
        operation: "document.save".into(),
        payload: serde_json::json!({}),
        priority: 0,
        status: JobStatus::Pending,
        idempotency_key: key.into(),
        depends_on,
        attempts: 0,
        max_attempts: 3,
        run_at: now,
        deadline: None,
        causation_id: None,
        causation_depth: 0,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn utf8_offsets_snap_to_char_boundary() {
    // “雾在潮响” 每字 3 字节；旧的字节偏移落在字符中间会 panic
    let mut text = String::from("雾在潮响");
    novel_storage::apply_operation_for_test(
        &mut text,
        &novel_domain::TextOperation::Insert {
            block_id: Default::default(),
            offset: 1, // “雾”内部 → 对齐到 0
            text: "风".into(),
        },
    )
    .unwrap();
    assert_eq!(text, "风雾在潮响");

    let mut text = String::from("雾在潮响");
    novel_storage::apply_operation_for_test(
        &mut text,
        &novel_domain::TextOperation::Insert {
            block_id: Default::default(),
            offset: 4, // “在”内部 → 对齐到 3
            text: "风".into(),
        },
    )
    .unwrap();
    assert_eq!(text, "雾风在潮响");

    let mut text = String::from("雾在潮响");
    novel_storage::apply_operation_for_test(
        &mut text,
        &novel_domain::TextOperation::Delete {
            block_id: Default::default(),
            offset: 1, // “雾”内部 → 0
            length: 3, // 到字节 3（“在”起点），恰好删掉“雾”
        },
    )
    .unwrap();
    assert_eq!(text, "在潮响");

    // 超长偏移被夹取到文末（追加语义）
    let mut text = String::from("雾在");
    novel_storage::apply_operation_for_test(
        &mut text,
        &novel_domain::TextOperation::Insert {
            block_id: Default::default(),
            offset: u32::MAX,
            text: "潮响".into(),
        },
    )
    .unwrap();
    assert_eq!(text, "雾在潮响");
}

#[test]
fn create_chapter_rejects_unknown_book() {
    let repository = Repository::open_in_memory().unwrap();
    let project = repository.create_project("测试").unwrap();
    let error = repository
        .create_chapter(
            &project.id,
            "00000000-0000-0000-0000-000000000009",
            "第一章",
            1,
        )
        .unwrap_err();
    assert!(error.to_string().contains("not found"), "{error}");
}

#[test]
fn claim_complete_and_fail_lifecycle() {
    let repository = Repository::open_in_memory().unwrap();
    repository
        .enqueue_job(&pending_job("lifecycle", vec![]))
        .unwrap();

    let now = Utc::now();
    let job = repository
        .claim_next_job(now, Duration::minutes(10))
        .unwrap()
        .expect("应领取到任务");
    assert_eq!(job.status, JobStatus::Running);
    assert_eq!(job.attempts, 1);

    // 领取后不再有可领取任务
    assert!(repository
        .claim_next_job(now + Duration::seconds(1), Duration::minutes(10))
        .unwrap()
        .is_none());

    // 失败 → 回到 pending 并延后
    let dead = repository
        .fail_job(&job, "boom", Duration::seconds(5), now)
        .unwrap();
    assert!(!dead);
    let jobs = repository.list_jobs(10).unwrap();
    assert_eq!(jobs[0].status, JobStatus::Pending);
    assert_eq!(jobs[0].attempts, 1);
    assert!(jobs[0].run_at > now);

    // run_at 未到不可领取
    assert!(repository
        .claim_next_job(now + Duration::seconds(1), Duration::minutes(10))
        .unwrap()
        .is_none());
    // 到期后可再次领取，attempts 递增
    let job = repository
        .claim_next_job(now + Duration::seconds(30), Duration::minutes(10))
        .unwrap()
        .expect("退避到期应可领取");
    assert_eq!(job.attempts, 2);

    // 第三次失败达到 max_attempts → 死信
    let dead = repository
        .fail_job(&job, "boom", Duration::seconds(5), now)
        .unwrap();
    assert!(!dead);
    let job = repository
        .claim_next_job(now + Duration::seconds(30), Duration::minutes(10))
        .unwrap()
        .unwrap();
    let dead = repository
        .fail_job(&job, "boom", Duration::seconds(5), now)
        .unwrap();
    assert!(dead);
    assert_eq!(
        repository.list_jobs(10).unwrap()[0].status,
        JobStatus::DeadLetter
    );

    // 成功路径
    repository
        .complete_job(&job.id, &serde_json::json!({"ok": true}), now)
        .unwrap();
}

#[test]
fn stale_running_job_is_reclaimed() {
    let repository = Repository::open_in_memory().unwrap();
    let mut job = pending_job("stale", vec![]);
    job.status = JobStatus::Running;
    job.attempts = 1;
    job.updated_at = Utc::now() - Duration::hours(1);
    repository.enqueue_job(&job).unwrap();

    // 超过 stale 阈值的 running 任务被回收并重新领取
    let claimed = repository
        .claim_next_job(Utc::now(), Duration::minutes(10))
        .unwrap()
        .expect("陈旧任务应被回收领取");
    assert_eq!(claimed.attempts, 2);
}

#[test]
fn dependencies_block_claiming() {
    let repository = Repository::open_in_memory().unwrap();
    let blocker = pending_job("blocker", vec![]);
    let dependent = pending_job("dependent", vec![blocker.id.clone()]);
    repository.enqueue_job(&blocker).unwrap();
    repository.enqueue_job(&dependent).unwrap();

    let now = Utc::now();
    // 低优先级 dependent 先被扫到也不能领取（依赖未成功）
    let claimed = repository
        .claim_next_job(now, Duration::minutes(10))
        .unwrap()
        .expect("应领取无依赖的 blocker");
    assert_eq!(claimed.idempotency_key, "blocker");

    // blocker 完成前 dependent 不可领取
    repository
        .complete_job(&claimed.id, &serde_json::json!({}), now)
        .unwrap();
    let claimed = repository
        .claim_next_job(now + Duration::seconds(1), Duration::minutes(10))
        .unwrap()
        .expect("依赖满足后应可领取");
    assert_eq!(claimed.idempotency_key, "dependent");
}

#[test]
fn workflow_cooldown_roundtrip() {
    let repository = Repository::open_in_memory().unwrap();
    let rule = WorkflowRule {
        id: Default::default(),
        project_id: ProjectId::new(),
        name: "冷却规则".into(),
        enabled: true,
        trigger: WorkflowTrigger {
            event_type: "editor.idle".into(),
        },
        conditions: vec![],
        actions: vec![],
        priority: 0,
        cooldown_ms: 60_000,
    };

    let now = Utc::now();
    assert!(!repository
        .workflow_in_cooldown(&rule, "editor.idle", now)
        .unwrap());
    repository
        .record_workflow_fired(&rule.id, "editor.idle", now)
        .unwrap();
    assert!(repository
        .workflow_in_cooldown(&rule, "editor.idle", now + Duration::seconds(30))
        .unwrap());
    assert!(!repository
        .workflow_in_cooldown(&rule, "editor.idle", now + Duration::seconds(61))
        .unwrap());

    // cooldown_ms = 0 视为不冷却
    let no_cooldown = WorkflowRule {
        cooldown_ms: 0,
        ..rule
    };
    assert!(!repository
        .workflow_in_cooldown(&no_cooldown, "editor.idle", now)
        .unwrap());

    let _ = WorkflowCondition {
        path: String::new(),
        operator: novel_domain::ConditionOperator::Eq,
        value: serde_json::json!(null),
    };
}
