-- 正史实体/事实归属到作品，便于按项目审核与过滤。
ALTER TABLE canon_entities ADD COLUMN project_id TEXT NOT NULL DEFAULT '';
ALTER TABLE canon_facts ADD COLUMN project_id TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_canon_entities_project ON canon_entities(project_id);
CREATE INDEX IF NOT EXISTS idx_canon_facts_project_status ON canon_facts(project_id, status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_canon_entities_project_name_kind
    ON canon_entities(project_id, canonical_name, kind);
