use crate::{BlockId, BookId, ChapterId, ProjectId, Revision, SceneId, VolumeId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: ProjectId,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 作品库快照：当前作品下的书与章，供 IPC / UI 一次拉取。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub projects: Vec<Project>,
    pub active_project_id: Option<String>,
    pub books: Vec<Book>,
    #[serde(default)]
    pub volumes: Vec<Volume>,
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub scenes: Vec<Scene>,
}

/// 章节正文 + 块序列，对应 `load_chapter` / `save_chapter`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterBody {
    pub chapter_id: String,
    pub revision: u64,
    pub text: String,
    #[serde(default)]
    pub blocks: Vec<ContentBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: BookId,
    pub project_id: ProjectId,
    pub title: String,
    pub synopsis: String,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub id: VolumeId,
    pub book_id: BookId,
    pub title: String,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub id: ChapterId,
    pub book_id: BookId,
    pub volume_id: Option<VolumeId>,
    pub title: String,
    pub position: u32,
    pub current_revision: Revision,
    pub status: ChapterStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChapterStatus {
    Draft,
    Completed,
    Archived,
}

/// 章内场次：作者预先写的一场戏标题，不替代正文。
/// `pov_entry_id` 可选，指向人物结构条目 id（不是启发式正史实体）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: SceneId,
    pub chapter_id: ChapterId,
    pub title: String,
    pub position: u32,
    #[serde(default)]
    pub pov_entry_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentFormat {
    PlainText,
    StructuredAst,
}

/// 块类型：正文块 vs 思考块（模拟 AI reasoning + output 的写作模型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockKind {
    Body,
    Thinking,
}

/// 思考块内的标记引用：调用系统中的任务、设定或自定义功能。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MarkupRef {
    Task {
        id: String,
        label: String,
        status: String,
    },
    Setting {
        entity_path: String,
        field: String,
        value: String,
    },
    Custom {
        tag: String,
        body: String,
    },
}

impl MarkupRef {
    /// 标记的纯文本摘要（导出训练数据时嵌入思考过程）。
    pub fn summary(&self) -> String {
        match self {
            MarkupRef::Task { label, status, .. } => format!("任务[{status}]: {label}"),
            MarkupRef::Setting {
                entity_path,
                field,
                value,
            } => format!("设定 {entity_path}.{field} = {value}"),
            MarkupRef::Custom { tag, body } => format!("标记 {tag}: {body}"),
        }
    }
}

/// 结构化内容块：块级写作模型的最小单位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentBlock {
    pub id: BlockId,
    pub kind: BlockKind,
    pub text: String,
    pub position: u32,
    #[serde(default)]
    pub markup: Vec<MarkupRef>,
}

/// 块序列快照：替代纯文本快照的结构化内容（对应 ContentFormat::StructuredAst）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockSequence {
    pub chapter_id: ChapterId,
    pub revision: Revision,
    pub blocks: Vec<ContentBlock>,
    pub created_at: DateTime<Utc>,
}

impl BlockSequence {
    /// 导出用的正文纯文本（忽略思考块与标记）。
    pub fn body_text(&self) -> String {
        self.blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Body)
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSnapshot {
    pub chapter_id: ChapterId,
    pub revision: Revision,
    pub format: ContentFormat,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TextOperation {
    Insert {
        block_id: BlockId,
        offset: u32,
        text: String,
    },
    Delete {
        block_id: BlockId,
        offset: u32,
        length: u32,
    },
    CreateBlock {
        block_id: BlockId,
        parent: Option<BlockId>,
        position: u32,
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationAnchor {
    pub block_id: BlockId,
    pub base_revision: Revision,
    pub start_offset: u32,
    pub end_offset: u32,
    pub quote: String,
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub id: crate::AnnotationId,
    pub project_id: ProjectId,
    pub chapter_id: ChapterId,
    pub anchor: AnnotationAnchor,
    pub kind: AnnotationKind,
    pub body: String,
    pub resolved: bool,
    pub outdated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnnotationKind {
    RangeComment,
    BlockNote,
    SceneCard,
    AgentSuggestion,
    ThinkingMarkup,
}
