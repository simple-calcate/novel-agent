use super::{parse_project_id, Repository};
use crate::StorageError;
use novel_domain::{DomainError, ProjectId, StoryEntry, StoryEntryKind};
use rusqlite::params;
use uuid::Uuid;

impl Repository {
    pub fn create_story_entry(
        &self,
        project_id: &ProjectId,
        kind: StoryEntryKind,
        title: &str,
        summary: &str,
    ) -> Result<StoryEntry, StorageError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(DomainError::Validation("title required".into()).into());
        }
        let summary = summary.trim().to_owned();
        let entry = StoryEntry {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.clone(),
            kind,
            title: title.to_owned(),
            summary,
        };
        let inserted = self.connection.execute(
            "INSERT OR IGNORE INTO story_entries(id, project_id, kind, title, summary)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entry.id,
                project_id.to_string(),
                kind_name(kind),
                entry.title,
                entry.summary,
            ],
        )?;
        if inserted == 0 {
            return Err(DomainError::Validation(format!(
                "structure entry already exists: {title}"
            ))
            .into());
        }
        Ok(entry)
    }

    pub fn list_story_entries(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<StoryEntry>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, kind, title, summary
             FROM story_entries
             WHERE project_id = ?1
             ORDER BY CASE kind
                 WHEN 'character' THEN 0
                 WHEN 'setting' THEN 1
                 ELSE 2
             END, title",
        )?;
        let mut rows = statement.query(params![project_id.to_string()])?;
        let mut entries = Vec::new();
        while let Some(row) = rows.next()? {
            let kind: String = row.get(2)?;
            let Some(kind) = parse_kind(&kind) else {
                continue;
            };
            let project: String = row.get(1)?;
            entries.push(StoryEntry {
                id: row.get(0)?,
                project_id: parse_project_id(&project)?,
                kind,
                title: row.get(3)?,
                summary: row.get(4)?,
            });
        }
        Ok(entries)
    }

    pub fn delete_story_entry(
        &self,
        project_id: &ProjectId,
        id: &str,
        kind: StoryEntryKind,
    ) -> Result<(), StorageError> {
        let deleted = self.connection.execute(
            "DELETE FROM story_entries WHERE id = ?1 AND project_id = ?2 AND kind = ?3",
            params![id, project_id.to_string(), kind_name(kind)],
        )?;
        if deleted == 0 {
            return Err(DomainError::NotFound(format!("story entry {id}")).into());
        }
        Ok(())
    }
}

fn kind_name(kind: StoryEntryKind) -> &'static str {
    match kind {
        StoryEntryKind::Character => "character",
        StoryEntryKind::Setting => "setting",
        StoryEntryKind::Foreshadow => "foreshadow",
    }
}

fn parse_kind(value: &str) -> Option<StoryEntryKind> {
    match value {
        "character" => Some(StoryEntryKind::Character),
        "setting" => Some(StoryEntryKind::Setting),
        "foreshadow" => Some(StoryEntryKind::Foreshadow),
        _ => None,
    }
}
