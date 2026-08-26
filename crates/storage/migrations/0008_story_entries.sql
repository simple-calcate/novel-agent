CREATE TABLE story_entries (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    title TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_story_entries_project ON story_entries(project_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_story_entries_unique
    ON story_entries(project_id, kind, title);
