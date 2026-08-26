//! 应用层作品库：写库结束后才 dispatch，订阅者可以再次进入单写者。

use novel_domain::{DomainEvent, EventKind};
use novel_extensions::{BuiltinsExtension, Workspace};
use novel_kernel::{EventSubscriber, Kernel, KernelError};
use novel_storage::StorageHandle;
use serde_json::{json, Value};
use std::sync::Arc;

struct TouchStorage;

impl EventSubscriber for TouchStorage {
    fn id(&self) -> &str {
        "test.touch-storage"
    }

    fn event_types(&self) -> &[&str] {
        &["project.created"]
    }

    fn handle(&self, kernel: &Kernel, _event: &DomainEvent) -> Result<Value, KernelError> {
        novel_extensions::util::with_repository(kernel, |repository| repository.list_projects())?;
        Ok(json!({ "touched": true }))
    }
}

fn kernel_with_touch() -> Kernel {
    Kernel::builder()
        .service(Arc::new(StorageHandle::open_in_memory().unwrap()))
        .subscriber(TouchStorage)
        .extension(BuiltinsExtension)
        .expect("内置扩展")
        .build()
        .unwrap()
}

#[test]
fn create_project_dispatches_after_write() {
    let kernel = kernel_with_touch();
    let workspace = Workspace::new(&kernel);
    let project = workspace.create_project("夜航星图").unwrap();
    let snapshot = workspace.load_library(Some(project.id.clone())).unwrap();
    assert_eq!(snapshot.projects.len(), 1);
    assert_eq!(snapshot.projects[0].title, "夜航星图");
    assert_eq!(
        snapshot.active_project_id.as_deref(),
        Some(project.id.to_string().as_str())
    );
}

#[test]
fn book_and_chapter_roundtrip() {
    let kernel = kernel_with_touch();
    let workspace = Workspace::new(&kernel);
    let project = workspace.create_project("作品").unwrap();
    let book = workspace.create_book(&project.id, "卷一", "", 0).unwrap();
    let chapter = workspace
        .create_chapter(&project.id, &book.id.to_string(), "第一章", 0)
        .unwrap();
    workspace
        .save_chapter(&chapter.id, "雾港来客。", None)
        .unwrap();
    let body = workspace.load_chapter(&chapter.id).unwrap();
    assert_eq!(body.text, "雾港来客。");
    assert_eq!(body.revision, 1);
}

#[test]
fn propose_and_review_canon_from_chapter() {
    let kernel = kernel_with_touch();
    let workspace = Workspace::new(&kernel);
    let project = workspace.create_project("作品").unwrap();
    let book = workspace.create_book(&project.id, "卷一", "", 0).unwrap();
    let chapter = workspace
        .create_chapter(&project.id, &book.id.to_string(), "第一章", 0)
        .unwrap();
    workspace
        .save_chapter(&chapter.id, "林晚说道：「今夜雾很重。」", None)
        .unwrap();
    let created = workspace.propose_canon_from_chapter(&chapter.id).unwrap();
    assert!(!created.is_empty());
    let candidates = workspace
        .list_canon(&project.id, Some(novel_domain::FactStatus::Candidate))
        .unwrap();
    assert_eq!(candidates.len(), created.len());
    let reviewed = workspace
        .review_canon_fact(&candidates[0].fact_id, true)
        .unwrap();
    assert_eq!(reviewed.status, novel_domain::FactStatus::Accepted);
    assert!(workspace
        .list_canon(&project.id, Some(novel_domain::FactStatus::Candidate))
        .unwrap()
        .iter()
        .all(|item| item.fact_id != reviewed.fact_id));
}

#[test]
fn user_event_kind_matches_library_ops() {
    assert_eq!(EventKind::ProjectCreated.as_str(), "project.created");
    assert_eq!(EventKind::BookCreated.as_str(), "book.created");
    assert_eq!(EventKind::ChapterCreated.as_str(), "chapter.created");
    assert_eq!(EventKind::CanonProposed.as_str(), "canon.proposed");
    assert_eq!(EventKind::CanonAccepted.as_str(), "canon.accepted");
}
