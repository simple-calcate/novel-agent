//! 块级写作扩展：结构化块编辑（block.save / block.edit）与
//! 按写作协议导出训练数据（training.export）。
//!
//! 拍级配对与质量分级的权威实现见 `novel_domain::protocol`。

use crate::util::{with_repository, with_repository_mut};
use async_trait::async_trait;
use novel_domain::{
    build_training_examples, filter_examples, quality_counts, serialize_examples, BlockId,
    BlockKind, ChapterId, ContentBlock, ExampleQuality, MarkupRef, ProjectId, Revision,
    WRITING_PROTOCOL_VERSION,
};
use novel_kernel::{Extension, KernelBuilder, KernelError, Tool, ToolContext};
use serde::Deserialize;
use serde_json::{json, Value};

fn string_field(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn project_id(input: &Value) -> Result<ProjectId, KernelError> {
    string_field(input, "projectId")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| KernelError::ToolFailed {
            tool: "block".into(),
            message: "payload 缺少合法的 projectId".into(),
        })
}

fn chapter_id(input: &Value) -> Result<ChapterId, KernelError> {
    string_field(input, "chapterId")
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| KernelError::ToolFailed {
            tool: "block".into(),
            message: "payload 缺少合法的 chapterId".into(),
        })
}

fn base_revision(input: &Value) -> Result<Revision, KernelError> {
    input
        .get("baseRevision")
        .and_then(Value::as_u64)
        .map(Revision)
        .ok_or_else(|| KernelError::ToolFailed {
            tool: "block".into(),
            message: "payload 缺少 baseRevision".into(),
        })
}

/// 把 JSON 数组解析为按 position 稳定排序、重新编号的块列表。
fn parse_blocks(value: &Value) -> Result<Vec<ContentBlock>, KernelError> {
    let items = value.as_array().ok_or_else(|| KernelError::ToolFailed {
        tool: "block".into(),
        message: "blocks 必须是数组".into(),
    })?;
    let mut blocks = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let id: BlockId = item
            .get("id")
            .and_then(Value::as_str)
            .and_then(|value| value.parse().ok())
            .ok_or_else(|| KernelError::ToolFailed {
                tool: "block".into(),
                message: format!("blocks[{index}] 缺少合法的 id"),
            })?;
        let kind = match item.get("kind").and_then(Value::as_str) {
            Some("thinking") => BlockKind::Thinking,
            _ => BlockKind::Body,
        };
        let markup = item
            .get("markup")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|m| serde_json::from_value::<MarkupRef>(m.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        blocks.push(ContentBlock {
            id,
            kind,
            text: item
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            position: item
                .get("position")
                .and_then(Value::as_u64)
                .unwrap_or(index as u64) as u32,
            markup,
        });
    }
    blocks.sort_by_key(|block| block.position);
    for (index, block) in blocks.iter_mut().enumerate() {
        block.position = index as u32;
    }
    Ok(blocks)
}

/// 校验章节归属项目，避免跨项目写入。
fn ensure_chapter_belongs(
    repository: &novel_storage::Repository,
    chapter_id: &ChapterId,
    project_id: &ProjectId,
) -> Result<(), novel_storage::StorageError> {
    let belongs = repository.chapter_project_id(chapter_id)?;
    if belongs.as_ref() != Some(project_id) {
        return Err(novel_domain::DomainError::NotFound(format!(
            "chapter {chapter_id} in project {project_id}"
        ))
        .into());
    }
    Ok(())
}

/// 块级保存：编辑器提交整章块状态（冲突检测基于 baseRevision）。
pub struct BlockSaveTool;

#[async_trait]
impl Tool for BlockSaveTool {
    fn id(&self) -> &str {
        "block.save"
    }

    fn summary(&self) -> &str {
        "提交一章的完整块序列（正文/思考块），带版本冲突检测"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = project_id(&input)?;
        let chapter_id = chapter_id(&input)?;
        let revision = base_revision(&input)?;
        let blocks = parse_blocks(&input)?;

        let next = with_repository_mut(ctx.kernel(), |repository| {
            ensure_chapter_belongs(repository, &chapter_id, &project_id)?;
            repository.commit_block_sequence(&chapter_id, revision, &blocks)
        })?;

        Ok(json!({
            "saved": true,
            "revision": next.0,
            "blockCount": blocks.len(),
        }))
    }
}

/// 细粒度块操作（插件/工作流使用）。
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
enum BlockOp {
    Create {
        #[serde(default)]
        block_id: Option<String>,
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        position: Option<u32>,
        #[serde(default)]
        text: String,
        #[serde(default)]
        markup: Vec<MarkupRef>,
    },
    Update {
        block_id: String,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        kind: Option<String>,
        #[serde(default)]
        markup: Option<Vec<MarkupRef>>,
    },
    Delete {
        block_id: String,
    },
    Move {
        block_id: String,
        position: u32,
    },
    SetMarkup {
        block_id: String,
        markup: Vec<MarkupRef>,
    },
}

fn apply_block_ops(blocks: &mut Vec<ContentBlock>, ops: &[BlockOp]) -> Result<(), KernelError> {
    for op in ops {
        match op {
            BlockOp::Create {
                block_id,
                kind,
                position,
                text,
                markup,
            } => {
                let id = block_id
                    .as_ref()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or_else(BlockId::new);
                let kind = match kind.as_deref() {
                    Some("thinking") => BlockKind::Thinking,
                    _ => BlockKind::Body,
                };
                let position = position
                    .unwrap_or(blocks.len() as u32)
                    .min(blocks.len() as u32) as usize;
                blocks.insert(
                    position,
                    ContentBlock {
                        id,
                        kind,
                        text: text.clone(),
                        position: position as u32,
                        markup: markup.clone(),
                    },
                );
            }
            BlockOp::Update {
                block_id,
                text,
                kind,
                markup,
            } => {
                let block = blocks
                    .iter_mut()
                    .find(|block| &block.id.to_string() == block_id)
                    .ok_or_else(|| block_not_found(block_id))?;
                if let Some(text) = text {
                    block.text = text.clone();
                }
                if let Some(kind) = kind {
                    block.kind = if kind == "thinking" {
                        BlockKind::Thinking
                    } else {
                        BlockKind::Body
                    };
                }
                if let Some(markup) = markup {
                    block.markup = markup.clone();
                }
            }
            BlockOp::Delete { block_id } => {
                let index = blocks
                    .iter()
                    .position(|block| &block.id.to_string() == block_id)
                    .ok_or_else(|| block_not_found(block_id))?;
                blocks.remove(index);
            }
            BlockOp::Move { block_id, position } => {
                let index = blocks
                    .iter()
                    .position(|block| &block.id.to_string() == block_id)
                    .ok_or_else(|| block_not_found(block_id))?;
                let block = blocks.remove(index);
                let position = (*position).min(blocks.len() as u32) as usize;
                blocks.insert(position, block);
            }
            BlockOp::SetMarkup { block_id, markup } => {
                let block = blocks
                    .iter_mut()
                    .find(|block| &block.id.to_string() == block_id)
                    .ok_or_else(|| block_not_found(block_id))?;
                block.markup = markup.clone();
            }
        }
    }
    for (index, block) in blocks.iter_mut().enumerate() {
        block.position = index as u32;
    }
    Ok(())
}

fn block_not_found(block_id: &str) -> KernelError {
    KernelError::ToolFailed {
        tool: "block.edit".into(),
        message: format!("block not found: {block_id}"),
    }
}

/// 块级编辑：在最新版本上应用操作，供插件与工作流细粒度修改块。
pub struct BlockEditTool;

#[async_trait]
impl Tool for BlockEditTool {
    fn id(&self) -> &str {
        "block.edit"
    }

    fn summary(&self) -> &str {
        "在章节最新块序列上应用 create/update/delete/move/setMarkup 操作"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = project_id(&input)?;
        let chapter_id = chapter_id(&input)?;
        let ops: Vec<BlockOp> = input
            .get("operations")
            .and_then(Value::as_array)
            .ok_or_else(|| KernelError::ToolFailed {
                tool: "block.edit".into(),
                message: "payload 缺少 operations 数组".into(),
            })?
            .iter()
            .map(|item| serde_json::from_value(item.clone()))
            .collect::<Result<_, _>>()
            .map_err(|error| KernelError::ToolFailed {
                tool: "block.edit".into(),
                message: format!("解析操作失败: {error}"),
            })?;

        let (mut blocks, base) = with_repository_mut(ctx.kernel(), |repository| {
            ensure_chapter_belongs(repository, &chapter_id, &project_id)?;
            let base = repository.current_revision(&chapter_id)?;
            let blocks = repository
                .latest_block_sequence(&chapter_id)?
                .map(|sequence| sequence.blocks)
                .unwrap_or_default();
            Ok((blocks, base))
        })?;

        apply_block_ops(&mut blocks, &ops)?;

        let next = with_repository_mut(ctx.kernel(), |repository| {
            repository.commit_block_sequence(&chapter_id, base, &blocks)
        })?;

        Ok(json!({
            "saved": true,
            "revision": next.0,
            "blockCount": blocks.len(),
        }))
    }
}

/// 一键导出训练数据：按写作协议把章节切成拍级样本并分级。
pub struct TrainingExportTool;

#[async_trait]
impl Tool for TrainingExportTool {
    fn id(&self) -> &str {
        "training.export"
    }

    fn summary(&self) -> &str {
        "按写作协议导出拍级训练数据（jsonl/sharegpt/alpaca/r1），默认丢弃 skip"
    }

    async fn execute(&self, input: Value, ctx: &ToolContext<'_>) -> Result<Value, KernelError> {
        let project_id = project_id(&input)?;
        let chapter_id = chapter_id(&input)?;
        let format = string_field(&input, "format").unwrap_or_else(|| "jsonl".into());
        let include_markup = input
            .get("includeMarkup")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let min_quality = string_field(&input, "minQuality")
            .as_deref()
            .and_then(ExampleQuality::parse)
            .unwrap_or(ExampleQuality::Usable);
        let revision = input.get("revision").and_then(Value::as_u64);

        let (sequence, chapter_title) = with_repository(ctx.kernel(), |repository| {
            ensure_chapter_belongs(repository, &chapter_id, &project_id)?;
            let title = repository
                .list_chapters(&project_id)?
                .into_iter()
                .find(|chapter| chapter.id == chapter_id)
                .map(|chapter| chapter.title);
            let sequence = match revision {
                Some(value) => repository.block_sequence(&chapter_id, Revision(value))?,
                None => repository.latest_block_sequence(&chapter_id)?,
            };
            Ok((sequence, title))
        })?;

        let Some(sequence) = sequence else {
            return Ok(json!({
                "format": format,
                "protocolVersion": WRITING_PROTOCOL_VERSION,
                "examples": [],
                "output": "",
                "revision": null,
                "dropped": 0,
                "qualityCounts": { "gold": 0, "usable": 0, "skip": 0 },
            }));
        };

        let all = build_training_examples(&sequence, include_markup, chapter_title.as_deref());
        let counts = quality_counts(&all);
        let examples = filter_examples(&all, min_quality);
        let dropped = all.len().saturating_sub(examples.len());
        let output =
            serialize_examples(&examples, &format).map_err(|message| KernelError::ToolFailed {
                tool: "training.export".into(),
                message,
            })?;

        Ok(json!({
            "format": format,
            "protocolVersion": WRITING_PROTOCOL_VERSION,
            "minQuality": min_quality.as_str(),
            "examples": examples,
            "output": output,
            "revision": sequence.revision.0,
            "dropped": dropped,
            "qualityCounts": counts,
        }))
    }
}

/// 块级写作扩展。
pub struct BlocksExtension;

impl Extension for BlocksExtension {
    fn id(&self) -> &str {
        "builtin.blocks"
    }

    fn setup(&self, builder: &mut KernelBuilder) -> Result<(), KernelError> {
        builder.register_tool(BlockSaveTool);
        builder.register_tool(BlockEditTool);
        builder.register_tool(TrainingExportTool);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use novel_domain::BlockId;

    /// 把测试标签确定性映射为合法 UUID，保证 id 定位断言可复现。
    fn tag_uuid(tag: &str) -> String {
        let mut bytes = [0u8; 16];
        let n = tag.len().min(16);
        bytes[..n].copy_from_slice(&tag.as_bytes()[..n]);
        uuid::Uuid::from_bytes(bytes).to_string()
    }

    fn block(id: &str, kind: BlockKind, text: &str) -> ContentBlock {
        ContentBlock {
            id: BlockId(uuid::Uuid::parse_str(&tag_uuid(id)).unwrap()),
            kind,
            text: text.to_owned(),
            position: 0,
            markup: vec![],
        }
    }

    #[test]
    fn ops_create_update_delete_move() {
        let mut blocks = vec![
            block("a", BlockKind::Body, "第一段"),
            block("b", BlockKind::Thinking, "思考"),
        ];
        apply_block_ops(
            &mut blocks,
            &[
                BlockOp::Create {
                    block_id: Some(tag_uuid("c")),
                    kind: Some("body".into()),
                    position: Some(1),
                    text: "插入段".into(),
                    markup: vec![],
                },
                BlockOp::Update {
                    block_id: tag_uuid("a"),
                    text: Some("改后".into()),
                    kind: None,
                    markup: None,
                },
            ],
        )
        .unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].text, "改后");
        assert_eq!(blocks[1].text, "插入段");
        assert_eq!(blocks[1].position, 1);

        apply_block_ops(
            &mut blocks,
            &[BlockOp::Move {
                block_id: tag_uuid("c"),
                position: 2,
            }],
        )
        .unwrap();
        assert_eq!(blocks[2].id.to_string(), tag_uuid("c"));

        apply_block_ops(
            &mut blocks,
            &[BlockOp::Delete {
                block_id: tag_uuid("b"),
            }],
        )
        .unwrap();
        assert_eq!(blocks.len(), 2);
        assert!(blocks
            .iter()
            .all(|block| block.id.to_string() != tag_uuid("b")));
    }
}
