use super::Repository;
use crate::StorageError;
use chrono::Utc;
use novel_domain::{
    Actor, BlockId, BlockKind, BlockSequence, ChapterId, ContentBlock, ContentPatch, DomainError,
    ProjectId, ProposalId, Revision, TextOperation,
};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

impl Repository {
    /// 提交块序列；内容（忽略块 id）未变则不升版本。
    pub fn save_block_sequence(
        &mut self,
        chapter_id: &ChapterId,
        blocks: &[ContentBlock],
    ) -> Result<Revision, StorageError> {
        let base = self.current_revision(chapter_id)?;
        if let Some(existing) = self.block_sequence(chapter_id, base)? {
            if blocks_content_eq(&existing.blocks, blocks) {
                return Ok(base);
            }
        } else if blocks.is_empty() {
            let text = self.chapter_text(chapter_id, base)?.unwrap_or_default();
            if text.is_empty() {
                return Ok(base);
            }
        }
        self.commit_block_sequence(chapter_id, base, blocks)
    }

    /// 把当前正文写成新版本。文本未变则不递增修订号。
    pub fn save_chapter_snapshot(
        &mut self,
        chapter_id: &ChapterId,
        text: &str,
        actor: &str,
    ) -> Result<Revision, StorageError> {
        let base = self.current_revision(chapter_id)?;
        let current = self.chapter_text(chapter_id, base)?.unwrap_or_default();
        if current == text {
            return Ok(base);
        }
        let block_id = BlockId::new();
        let mut operations = Vec::new();
        if !current.is_empty() {
            operations.push(TextOperation::Delete {
                block_id: block_id.clone(),
                offset: 0,
                length: current.len() as u32,
            });
        }
        if !text.is_empty() {
            operations.push(TextOperation::Insert {
                block_id,
                offset: 0,
                text: text.to_owned(),
            });
        }
        let patch = ContentPatch {
            id: ProposalId::new(),
            chapter_id: chapter_id.clone(),
            base_revision: base,
            operations: operations.clone(),
            rationale: "editor snapshot".into(),
            created_by: Actor::User { user_id: None },
            created_at: Utc::now(),
        };
        self.commit_patch(&patch, actor, &operations)
    }
    /// 章节 → 所属项目（operation_log 等处需要真实 project_id）。
    pub fn chapter_project_id(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Option<ProjectId>, StorageError> {
        let project_id: Option<String> = self
            .connection
            .query_row(
                "SELECT b.project_id FROM books b
                 JOIN chapters c ON c.book_id = b.id
                 WHERE c.id = ?1",
                [chapter_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        project_id
            .map(|value| {
                Uuid::parse_str(&value)
                    .map(ProjectId)
                    .map_err(|_| DomainError::Validation("bad project id".into()).into())
            })
            .transpose()
    }

    pub fn current_revision(&self, chapter_id: &ChapterId) -> Result<Revision, StorageError> {
        let revision: Option<i64> = self
            .connection
            .query_row(
                "SELECT current_revision FROM chapters WHERE id = ?1",
                [chapter_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        revision
            .map(|value| Revision(value as u64))
            .ok_or_else(|| DomainError::NotFound(format!("chapter {chapter_id}")).into())
    }

    /// 读取某个历史版本的正文字本。
    pub fn chapter_text(
        &self,
        chapter_id: &ChapterId,
        revision: Revision,
    ) -> Result<Option<String>, StorageError> {
        let text: Option<String> = self
            .connection
            .query_row(
                "SELECT text FROM revisions WHERE chapter_id = ?1 AND revision = ?2",
                params![chapter_id.to_string(), revision.0 as i64],
                |row| row.get(0),
            )
            .optional()?;
        Ok(text)
    }

    /// 某章节全部操作日志的 project_id（校验日志归属用）。
    pub fn operation_log_project_ids(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Vec<String>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT project_id FROM operation_log WHERE chapter_id = ?1 ORDER BY id")?;
        let rows = statement.query_map([chapter_id.to_string()], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn commit_patch(
        &mut self,
        patch: &ContentPatch,
        actor: &str,
        operations: &[TextOperation],
    ) -> Result<Revision, StorageError> {
        let transaction = self.connection.transaction()?;
        let actual: i64 = transaction.query_row(
            "SELECT current_revision FROM chapters WHERE id = ?1",
            [patch.chapter_id.to_string()],
            |row| row.get(0),
        )?;

        if actual as u64 != patch.base_revision.0 {
            return Err(DomainError::RevisionConflict {
                expected: patch.base_revision.0,
                actual: actual as u64,
            }
            .into());
        }

        let mut text: String = transaction
            .query_row(
                "SELECT text FROM revisions WHERE chapter_id = ?1 AND revision = ?2",
                params![patch.chapter_id.to_string(), actual],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_default();

        for operation in operations {
            apply_operation(&mut text, operation)?;
        }

        let next = Revision((actual as u64) + 1);
        let now = chrono::Utc::now().to_rfc3339();
        let project_id: Option<String> = transaction
            .query_row(
                "SELECT b.project_id FROM books b
                 JOIN chapters c ON c.book_id = b.id
                 WHERE c.id = ?1",
                [patch.chapter_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;
        transaction.execute(
            "INSERT INTO revisions(chapter_id, revision, format, text, created_at)
             VALUES (?1, ?2, 'plainText', ?3, ?4)",
            params![patch.chapter_id.to_string(), next.0 as i64, text, now],
        )?;
        transaction.execute(
            "UPDATE chapters SET current_revision = ?2 WHERE id = ?1",
            params![patch.chapter_id.to_string(), next.0 as i64],
        )?;
        transaction.execute(
            "INSERT INTO operation_log(
                project_id, chapter_id, revision_before, revision_after,
                actor, operations_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                project_id.unwrap_or_default(),
                patch.chapter_id.to_string(),
                actual,
                next.0 as i64,
                actor,
                serde_json::to_string(operations)?,
                now,
            ],
        )?;
        transaction.commit()?;
        Ok(next)
    }
    /// 提交块序列：校验 base_revision 冲突，写入新版本的块行，
    /// 同步纯文本快照（正文拼接）以兼容旧读取路径，更新章节版本并登记操作日志。
    pub fn commit_block_sequence(
        &mut self,
        chapter_id: &ChapterId,
        base_revision: Revision,
        blocks: &[ContentBlock],
    ) -> Result<Revision, StorageError> {
        let transaction = self.connection.transaction()?;
        let actual: i64 = transaction.query_row(
            "SELECT current_revision FROM chapters WHERE id = ?1",
            [chapter_id.to_string()],
            |row| row.get(0),
        )?;

        if actual as u64 != base_revision.0 {
            return Err(DomainError::RevisionConflict {
                expected: base_revision.0,
                actual: actual as u64,
            }
            .into());
        }

        let next = Revision((actual as u64) + 1);
        let now = chrono::Utc::now().to_rfc3339();

        transaction.execute(
            "DELETE FROM content_blocks WHERE chapter_id = ?1 AND revision = ?2",
            params![chapter_id.to_string(), next.0 as i64],
        )?;

        for block in blocks {
            transaction.execute(
                "INSERT INTO content_blocks(
                    id, chapter_id, revision, kind, position, text, markup_json, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    block.id.to_string(),
                    chapter_id.to_string(),
                    next.0 as i64,
                    match block.kind {
                        BlockKind::Body => "body",
                        BlockKind::Thinking => "thinking",
                    },
                    block.position,
                    block.text,
                    serde_json::to_string(&block.markup)?,
                    now,
                ],
            )?;
        }

        // 纯文本快照：正文拼接，兼容旧读取路径（chapter_text / DocumentSnapshot）。
        let body_text = blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Body)
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        transaction.execute(
            "INSERT INTO revisions(chapter_id, revision, format, text, created_at)
             VALUES (?1, ?2, 'structuredAst', ?3, ?4)",
            params![chapter_id.to_string(), next.0 as i64, body_text, now],
        )?;
        transaction.execute(
            "UPDATE chapters SET current_revision = ?2 WHERE id = ?1",
            params![chapter_id.to_string(), next.0 as i64],
        )?;

        let project_id: Option<String> = transaction
            .query_row(
                "SELECT b.project_id FROM books b
                 JOIN chapters c ON c.book_id = b.id
                 WHERE c.id = ?1",
                [chapter_id.to_string()],
                |row| row.get(0),
            )
            .optional()?;

        transaction.execute(
            "INSERT INTO operation_log(
                project_id, chapter_id, revision_before, revision_after,
                actor, operations_json, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                project_id.unwrap_or_default(),
                chapter_id.to_string(),
                actual,
                next.0 as i64,
                "editor",
                serde_json::to_string(&serde_json::json!({
                    "operation": "commitBlockSequence",
                    "blockCount": blocks.len(),
                    "blocks": blocks,
                }))?,
                now,
            ],
        )?;

        transaction.commit()?;
        Ok(next)
    }

    /// 读取指定版本的块序列；该版本无块数据时返回 None。
    pub fn block_sequence(
        &self,
        chapter_id: &ChapterId,
        revision: Revision,
    ) -> Result<Option<BlockSequence>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT id, kind, position, text, markup_json FROM content_blocks
             WHERE chapter_id = ?1 AND revision = ?2
             ORDER BY position ASC",
        )?;
        let rows =
            statement.query_map(params![chapter_id.to_string(), revision.0 as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u32>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?;

        let mut blocks = Vec::new();
        for row in rows {
            let (id, kind, position, text, markup_json) = row?;
            let Some(id) = id.parse().ok() else {
                continue;
            };
            let kind = match kind.as_str() {
                "body" => BlockKind::Body,
                "thinking" => BlockKind::Thinking,
                _ => continue,
            };
            blocks.push(ContentBlock {
                id,
                kind,
                text,
                position,
                markup: serde_json::from_str(&markup_json).unwrap_or_default(),
            });
        }

        if blocks.is_empty() {
            return Ok(None);
        }

        let created_at: Option<String> = self
            .connection
            .query_row(
                "SELECT created_at FROM revisions WHERE chapter_id = ?1 AND revision = ?2",
                params![chapter_id.to_string(), revision.0 as i64],
                |row| row.get(0),
            )
            .optional()?;
        let created_at = created_at
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
            .map(|date| date.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        Ok(Some(BlockSequence {
            chapter_id: chapter_id.clone(),
            revision,
            blocks,
            created_at,
        }))
    }

    /// 读取章节最新版本的块序列。
    pub fn latest_block_sequence(
        &self,
        chapter_id: &ChapterId,
    ) -> Result<Option<BlockSequence>, StorageError> {
        let revision = self.current_revision(chapter_id)?;
        self.block_sequence(chapter_id, revision)
    }
}

pub(super) fn apply_operation(
    text: &mut String,
    operation: &TextOperation,
) -> Result<(), StorageError> {
    match operation {
        TextOperation::Insert {
            offset,
            text: value,
            ..
        } => {
            // 偏移按字节解释，但必须对齐到字符边界，否则 UTF-8 中间会 panic。
            let offset = floor_char_boundary(text, *offset as usize);
            text.insert_str(offset, value);
        }
        TextOperation::Delete { offset, length, .. } => {
            let start = floor_char_boundary(text, *offset as usize);
            let end = floor_char_boundary(text, start.saturating_add(*length as usize));
            text.replace_range(start..end, "");
        }
        TextOperation::CreateBlock { text: value, .. } => {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(value);
        }
    }
    Ok(())
}

/// 把字节下标对齐到不大于它的字符边界（`str::floor_char_boundary` 的稳定实现）。
fn floor_char_boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn blocks_content_eq(left: &[ContentBlock], right: &[ContentBlock]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(a, b)| {
            a.kind == b.kind && a.text == b.text && a.position == b.position && a.markup == b.markup
        })
}
