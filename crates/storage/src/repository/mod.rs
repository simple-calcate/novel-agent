use crate::{run_migrations, StorageError};
use chrono::{DateTime, Utc};
use novel_domain::{BookId, ChapterStatus, DomainError, ProjectId};
use rusqlite::Connection;
use std::path::Path;

mod automation;
mod canon;
mod feedback;
mod library;
mod outbox;
mod queue;
mod revisions;
mod structure;

pub use outbox::OutboxEvent;

pub const SETTING_ACTIVE_PROJECT: &str = "active_project_id";

pub struct Repository {
    pub(in crate::repository) connection: Connection,
}

impl Repository {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let mut connection = Connection::open(path)?;
        run_migrations(&mut connection)?;
        Ok(Self { connection })
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let mut connection = Connection::open_in_memory()?;
        run_migrations(&mut connection)?;
        Ok(Self { connection })
    }

    pub fn save_setting(&self, key: &str, value: &str) -> Result<(), StorageError> {
        let now = chrono::Utc::now().to_rfc3339();
        self.connection.execute(
            "INSERT OR REPLACE INTO app_settings(key, value, updated_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![key, value, now],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT value FROM app_settings WHERE key = ?1")?;
        let mut rows = stmt.query(rusqlite::params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    pub(super) fn next_position(&self, sql: &str, key: &str) -> Result<u32, StorageError> {
        let max: i64 = self.connection.query_row(sql, [key], |row| row.get(0))?;
        Ok((max as u32).saturating_add(1))
    }

    pub(crate) fn transact<T>(
        &self,
        action: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        let value = action(&tx)?;
        tx.commit()?;
        Ok(value)
    }

    pub(crate) fn write_with_outbox<T>(
        &self,
        project_id: &str,
        event_type: &str,
        payload: serde_json::Value,
        action: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        self.transact(|tx| {
            let value = action(tx)?;
            outbox::insert(tx, project_id, event_type, &payload)?;
            Ok(value)
        })
    }
}

pub fn apply_operation_for_test(
    text: &mut String,
    operation: &novel_domain::TextOperation,
) -> Result<(), StorageError> {
    crate::repository::revisions::apply_operation(text, operation)
}

pub(super) fn parse_rfc3339(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

pub(super) fn parse_project_id(value: &str) -> Result<ProjectId, StorageError> {
    value
        .parse()
        .map_err(|_| DomainError::Validation("invalid project id".into()).into())
}

pub(super) fn parse_book_id(value: &str) -> Result<BookId, StorageError> {
    value
        .parse()
        .map_err(|_| DomainError::Validation("invalid book id".into()).into())
}

pub(super) fn chapter_status(name: &str) -> ChapterStatus {
    match name {
        "completed" => ChapterStatus::Completed,
        "archived" => ChapterStatus::Archived,
        _ => ChapterStatus::Draft,
    }
}
