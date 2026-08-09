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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub id: SceneId,
    pub chapter_id: ChapterId,
    pub title: String,
    pub position: u32,
    pub pov_entity_id: Option<crate::EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentFormat {
    PlainText,
    StructuredAst,
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
}
