CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE books (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    synopsis TEXT NOT NULL DEFAULT '',
    position INTEGER NOT NULL
);

CREATE TABLE volumes (
    id TEXT PRIMARY KEY,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    position INTEGER NOT NULL
);

CREATE TABLE chapters (
    id TEXT PRIMARY KEY,
    book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    volume_id TEXT REFERENCES volumes(id) ON DELETE SET NULL,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    current_revision INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'draft'
);

CREATE TABLE scenes (
    id TEXT PRIMARY KEY,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    position INTEGER NOT NULL,
    pov_entity_id TEXT
);

CREATE TABLE revisions (
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    format TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (chapter_id, revision)
);

CREATE TABLE operation_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    chapter_id TEXT,
    revision_before INTEGER NOT NULL,
    revision_after INTEGER NOT NULL,
    actor TEXT NOT NULL,
    operations_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE annotations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    chapter_id TEXT NOT NULL,
    anchor_json TEXT NOT NULL,
    kind TEXT NOT NULL,
    body TEXT NOT NULL,
    resolved INTEGER NOT NULL DEFAULT 0,
    outdated INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE domain_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    occurred_at TEXT NOT NULL,
    project_id TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    event_json TEXT NOT NULL
);

CREATE INDEX idx_domain_events_type ON domain_events(project_id, event_type, occurred_at);

CREATE TABLE workflows (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    cooldown_ms INTEGER NOT NULL DEFAULT 0,
    rule_json TEXT NOT NULL
);

CREATE TABLE jobs (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    workflow_id TEXT,
    operation TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    depends_on_json TEXT NOT NULL DEFAULT '[]',
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    run_at TEXT NOT NULL,
    deadline TEXT,
    causation_id TEXT,
    causation_depth INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_jobs_runnable ON jobs(status, run_at, priority DESC);

CREATE TABLE outbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    delivered_at TEXT
);

CREATE TABLE canon_entities (
    id TEXT PRIMARY KEY,
    branch_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    canonical_name TEXT NOT NULL,
    aliases_json TEXT NOT NULL,
    attributes_json TEXT NOT NULL
);

CREATE TABLE canon_facts (
    id TEXT PRIMARY KEY,
    entity_id TEXT NOT NULL REFERENCES canon_entities(id) ON DELETE CASCADE,
    branch_id TEXT NOT NULL,
    predicate TEXT NOT NULL,
    value_json TEXT NOT NULL,
    status TEXT NOT NULL,
    confidence REAL NOT NULL,
    source_json TEXT NOT NULL,
    fact_json TEXT NOT NULL
);

CREATE TABLE relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_entity TEXT NOT NULL,
    to_entity TEXT NOT NULL,
    relation TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    data_json TEXT NOT NULL
);

CREATE TABLE story_events (
    id TEXT PRIMARY KEY,
    branch_id TEXT NOT NULL,
    narrative_order INTEGER NOT NULL,
    data_json TEXT NOT NULL
);

CREATE TABLE character_knowledge (
    character_id TEXT NOT NULL,
    fact_id TEXT NOT NULL,
    learned_at_json TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    data_json TEXT NOT NULL,
    PRIMARY KEY (character_id, fact_id, branch_id)
);

CREATE TABLE plot_threads (
    id TEXT PRIMARY KEY,
    branch_id TEXT NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    data_json TEXT NOT NULL
);

CREATE TABLE agent_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    state_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE context_blocks (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    tier INTEGER NOT NULL,
    summary TEXT NOT NULL,
    block_json TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE preference_rules (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    scope TEXT NOT NULL,
    rule TEXT NOT NULL,
    status TEXT NOT NULL,
    data_json TEXT NOT NULL
);

CREATE TABLE plugins (
    id TEXT PRIMARY KEY,
    manifest_json TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE plugin_grants (
    plugin_id TEXT PRIMARY KEY REFERENCES plugins(id) ON DELETE CASCADE,
    grants_json TEXT NOT NULL
);

CREATE TABLE result_objects (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    content_type TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE VIRTUAL TABLE search_documents USING fts5(
    project_id UNINDEXED,
    entity_kind UNINDEXED,
    entity_id UNINDEXED,
    title,
    body,
    tokenize = 'unicode61'
);
