CREATE TABLE content_blocks_v2 (
    id TEXT NOT NULL,
    chapter_id TEXT NOT NULL REFERENCES chapters(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    kind TEXT NOT NULL,
    position INTEGER NOT NULL,
    text TEXT NOT NULL DEFAULT '',
    markup_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    PRIMARY KEY (chapter_id, revision, id)
);

INSERT INTO content_blocks_v2(
    id, chapter_id, revision, kind, position, text, markup_json, created_at
)
SELECT id, chapter_id, revision, kind, position, text, markup_json, created_at
FROM content_blocks;

DROP TABLE content_blocks;
ALTER TABLE content_blocks_v2 RENAME TO content_blocks;

CREATE INDEX idx_blocks_chapter ON content_blocks(chapter_id, revision, position);
