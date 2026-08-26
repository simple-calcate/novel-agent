use super::Repository;
use crate::StorageError;
use chrono::Utc;
use novel_domain::{Annotation, DomainError, DomainEvent, ProjectId, WorkflowId, WorkflowRule};
use rusqlite::{params, OptionalExtension};

impl Repository {
    pub fn record_event(&self, event: &DomainEvent) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO domain_events(
                id, event_type, schema_version, occurred_at, project_id, payload_json, event_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.event_id.to_string(),
                event.event_type,
                event.schema_version,
                event.occurred_at.to_rfc3339(),
                event.project_id.to_string(),
                serde_json::to_string(&event.payload)?,
                serde_json::to_string(event)?,
            ],
        )?;
        Ok(())
    }

    pub fn save_workflow(&self, rule: &WorkflowRule) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO workflows(
                id, project_id, name, enabled, priority, cooldown_ms, rule_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                rule.id.to_string(),
                rule.project_id.to_string(),
                rule.name,
                rule.enabled,
                rule.priority,
                rule.cooldown_ms as i64,
                serde_json::to_string(rule)?,
            ],
        )?;
        Ok(())
    }

    pub fn workflows_for_event(
        &self,
        project_id: &ProjectId,
        event_type: &str,
    ) -> Result<Vec<WorkflowRule>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT rule_json FROM workflows
             WHERE project_id = ?1 AND enabled = 1
             ORDER BY priority DESC",
        )?;
        let rows = statement.query_map([project_id.to_string()], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        let mut rules = Vec::new();
        for row in rows {
            let rule: WorkflowRule = serde_json::from_str(&row?)?;
            if rule.trigger.event_type == event_type {
                rules.push(rule);
            }
        }
        Ok(rules)
    }

    /// 工作流冷却检查：cooldown_ms 内已触发过则不再触发。
    pub fn workflow_in_cooldown(
        &self,
        rule: &WorkflowRule,
        event_type: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<bool, StorageError> {
        if rule.cooldown_ms == 0 {
            return Ok(false);
        }
        let last: Option<String> = self
            .connection
            .query_row(
                "SELECT last_fired_at FROM workflow_fired
                 WHERE workflow_id = ?1 AND event_type = ?2",
                params![rule.id.to_string(), event_type],
                |row| row.get(0),
            )
            .optional()?;
        let Some(last) = last else {
            return Ok(false);
        };
        let last = chrono::DateTime::parse_from_rfc3339(&last)
            .map_err(|_| DomainError::Validation("bad last_fired_at".into()))?
            .with_timezone(&chrono::Utc);
        Ok(now - last < chrono::Duration::milliseconds(rule.cooldown_ms as i64))
    }

    pub fn record_workflow_fired(
        &self,
        workflow_id: &WorkflowId,
        event_type: &str,
        now: chrono::DateTime<Utc>,
    ) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO workflow_fired(workflow_id, event_type, last_fired_at)
             VALUES (?1, ?2, ?3)",
            params![workflow_id.to_string(), event_type, now.to_rfc3339()],
        )?;
        Ok(())
    }
    pub fn save_result_object(
        &self,
        project_id: &ProjectId,
        content_type: &str,
        content: &str,
    ) -> Result<novel_domain::ResultObjectId, StorageError> {
        let id = novel_domain::ResultObjectId::new();
        self.connection.execute(
            "INSERT INTO result_objects(id, project_id, content_type, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id.to_string(),
                project_id.to_string(),
                content_type,
                content,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(id)
    }
    pub fn save_annotation(&self, annotation: &Annotation) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT OR REPLACE INTO annotations(
                id, project_id, chapter_id, anchor_json, kind, body, resolved, outdated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                annotation.id.to_string(),
                annotation.project_id.to_string(),
                annotation.chapter_id.to_string(),
                serde_json::to_string(&annotation.anchor)?,
                format!("{:?}", annotation.kind),
                annotation.body,
                annotation.resolved,
                annotation.outdated,
            ],
        )?;
        Ok(())
    }
}
