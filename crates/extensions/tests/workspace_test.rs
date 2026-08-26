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
fn user_event_kind_matches_library_ops() {
    assert_eq!(EventKind::ProjectCreated.as_str(), "project.created");
    assert_eq!(EventKind::BookCreated.as_str(), "book.created");
    assert_eq!(EventKind::ChapterCreated.as_str(), "chapter.created");
}
