use super::Repository;
use crate::StorageError;
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutboxEvent {
    pub id: i64,
    pub project_id: String,
    pub event_type: String,
    pub payload: Value,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

pub fn insert(
    conn: &Connection,
    project_id: &str,
    event_type: &str,
    payload: &Value,
) -> Result<i64, StorageError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO outbox(project_id, event_type, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![project_id, event_type, payload.to_string(), now],
    )?;
    Ok(conn.last_insert_rowid())
}

impl Repository {
    pub fn list_pending_outbox(&self, limit: u32) -> Result<Vec<OutboxEvent>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, event_type, payload_json, created_at, delivered_at
             FROM outbox
             WHERE delivered_at IS NULL
             ORDER BY id ASC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut events = Vec::new();
        for row in rows {
            let (id, project_id, event_type, payload_json, created_at, delivered_at) = row?;
            events.push(OutboxEvent {
                id,
                project_id,
                event_type,
                payload: serde_json::from_str(&payload_json).unwrap_or(Value::Null),
                created_at,
                delivered_at,
            });
        }
        Ok(events)
    }

    pub fn mark_outbox_delivered(&self, ids: &[i64]) -> Result<u32, StorageError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let now = Utc::now().to_rfc3339();
        let mut updated = 0u32;
        for id in ids {
            updated += self.connection.execute(
                "UPDATE outbox SET delivered_at = ?2 WHERE id = ?1 AND delivered_at IS NULL",
                params![id, now],
            )? as u32;
        }
        Ok(updated)
    }

    pub fn count_pending_outbox(&self) -> Result<u32, StorageError> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM outbox WHERE delivered_at IS NULL",
            [],
            |row| row.get(0),
        )?;
        Ok(count as u32)
    }
}
