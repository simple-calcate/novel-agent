use crate::{
    actor::{Actor, EventSource},
    BlockId, BookId, ChapterId, EventId, ProjectId, Revision, SceneId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const EVENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainEvent {
    pub event_id: EventId,
    pub event_type: String,
    pub schema_version: u32,
    pub occurred_at: DateTime<Utc>,
    pub project_id: ProjectId,
    pub book_id: Option<BookId>,
    pub chapter_id: Option<ChapterId>,
    pub scene_id: Option<SceneId>,
    pub block_id: Option<BlockId>,
    pub actor: Actor,
    pub source: EventSource,
    pub platform: Platform,
    pub transaction_id: String,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub revision_before: Revision,
    pub revision_after: Revision,
    pub payload: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Linux,
    Windows,
    Android,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    EditorIdle,
    EditorFocusChanged,
    SelectionChanged,
    ParagraphCreated,
    ParagraphSplit,
    SceneCreated,
    ProjectCreated,
    BookCreated,
    ChapterCreated,
    ChapterActivated,
    ChapterCompleted,
    ChapterReordered,
    ProjectRenamed,
    ProjectDeleted,
    BookRenamed,
    BookDeleted,
    BookReordered,
    ChapterRenamed,
    ChapterDeleted,
    ContentChanged,
    DocumentSaved,
    RevisionCommitted,
    AgentFinished,
    ProposalAccepted,
    ProposalRejected,
    ProposalCorrected,
    PreferenceConfirmed,
    JobFailed,
    PluginInstalled,
    SyncConnected,
    BlockModeChanged,
    Custom(String),
}

impl EventKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::EditorIdle => "editor.idle",
            Self::EditorFocusChanged => "editor.focus_changed",
            Self::SelectionChanged => "selection.changed",
            Self::ParagraphCreated => "paragraph.created",
            Self::ParagraphSplit => "paragraph.split",
            Self::SceneCreated => "scene.created",
            Self::ProjectCreated => "project.created",
            Self::BookCreated => "book.created",
            Self::ChapterCreated => "chapter.created",
            Self::ChapterActivated => "chapter.activated",
            Self::ChapterCompleted => "chapter.completed",
            Self::ChapterReordered => "chapter.reordered",
            Self::ProjectRenamed => "project.renamed",
            Self::ProjectDeleted => "project.deleted",
            Self::BookRenamed => "book.renamed",
            Self::BookDeleted => "book.deleted",
            Self::BookReordered => "book.reordered",
            Self::ChapterRenamed => "chapter.renamed",
            Self::ChapterDeleted => "chapter.deleted",
            Self::ContentChanged => "content.changed",
            Self::DocumentSaved => "document.saved",
            Self::RevisionCommitted => "revision.committed",
            Self::AgentFinished => "agent.finished",
            Self::ProposalAccepted => "proposal.accepted",
            Self::ProposalRejected => "proposal.rejected",
            Self::ProposalCorrected => "proposal.corrected",
            Self::PreferenceConfirmed => "preference.confirmed",
            Self::JobFailed => "job.failed",
            Self::PluginInstalled => "plugin.installed",
            Self::SyncConnected => "sync.connected",
            Self::BlockModeChanged => "block.mode.changed",
            Self::Custom(value) => value.as_str(),
        }
    }
}
