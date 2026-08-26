CREATE TABLE IF NOT EXISTS correction_records (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    proposal_id TEXT NOT NULL,
    ai_text TEXT NOT NULL,
    human_text TEXT NOT NULL,
    diff_summary TEXT NOT NULL,
    context_excerpt TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_correction_records_project ON correction_records(project_id, created_at);

CREATE INDEX IF NOT EXISTS idx_outbox_pending ON outbox(delivered_at, id);
