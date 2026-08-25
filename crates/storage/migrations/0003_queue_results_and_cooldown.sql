-- 0003: 队列执行结果落库 + 工作流冷却记录
ALTER TABLE jobs ADD COLUMN result_json TEXT;
ALTER TABLE jobs ADD COLUMN error TEXT;

CREATE TABLE workflow_fired (
    workflow_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    last_fired_at TEXT NOT NULL,
    PRIMARY KEY (workflow_id, event_type)
);
