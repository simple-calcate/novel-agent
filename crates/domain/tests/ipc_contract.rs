use novel_domain::{
    Book, Chapter, Project, RevisionDiff, RevisionSummary, Scene, StoryEntry, Volume,
};
use std::collections::BTreeSet;

const EXAMPLES: &str = include_str!("../../../packages/shared-types/examples.json");
const INTERFACES: &str = include_str!("../../../docs/interfaces.md");
const HOST: &str = include_str!("../../../apps/client/src-tauri/src/lib.rs");

#[test]
fn ipc_examples_deserialize_into_domain() {
    let value: serde_json::Value = serde_json::from_str(EXAMPLES).unwrap();
    let _: Project = serde_json::from_value(value["project"].clone()).unwrap();
    let _: Book = serde_json::from_value(value["book"].clone()).unwrap();
    let _: Volume = serde_json::from_value(value["volume"].clone()).unwrap();
    let _: Chapter = serde_json::from_value(value["chapter"].clone()).unwrap();
    let _: Scene = serde_json::from_value(value["scene"].clone()).unwrap();
    let _: StoryEntry = serde_json::from_value(value["storyEntry"].clone()).unwrap();
    let _: RevisionSummary = serde_json::from_value(value["revisionSummary"].clone()).unwrap();
    let _: RevisionDiff = serde_json::from_value(value["revisionDiff"].clone()).unwrap();
}

#[test]
fn interfaces_ipc_table_matches_generate_handler() {
    let documented = documented_ipc_commands(INTERFACES);
    let registered = generate_handler_commands(HOST);
    let only_docs: Vec<_> = documented.difference(&registered).cloned().collect();
    let only_code: Vec<_> = registered.difference(&documented).cloned().collect();
    assert!(
        only_docs.is_empty() && only_code.is_empty(),
        "docs/interfaces.md §5 与 generate_handler! 不一致\n只在文档: {only_docs:?}\n只在代码: {only_code:?}"
    );
    assert!(
        documented.len() >= 40,
        "命令表过短（{}），解析是否切错了章节",
        documented.len()
    );
}

fn documented_ipc_commands(markdown: &str) -> BTreeSet<String> {
    let section = markdown
        .split("## 5. 宿主 IPC")
        .nth(1)
        .expect("docs/interfaces.md 需要「## 5. 宿主 IPC」")
        .split("## 6.")
        .next()
        .expect("§5 之后需要「## 6.」");

    let mut names = BTreeSet::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let first = trimmed.split('|').nth(1).unwrap_or("").trim();
        if first.starts_with("命令") || first.starts_with("---") {
            continue;
        }
        for ident in backtick_idents(first) {
            if is_command_ident(&ident) {
                names.insert(ident);
            }
        }
    }
    names
}

fn generate_handler_commands(source: &str) -> BTreeSet<String> {
    const MARKER: &str = "generate_handler![";
    let start = source
        .find(MARKER)
        .expect("lib.rs 需要 tauri::generate_handler![...]");
    let body = &source[start + MARKER.len()..];
    let end = body.find(']').expect("generate_handler! 缺少 ]");
    body[..end]
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .map(str::trim)
        .filter(|ident| is_command_ident(ident))
        .map(ToOwned::to_owned)
        .collect()
}

fn backtick_idents(text: &str) -> Vec<String> {
    let mut idents = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('`') else {
            break;
        };
        idents.push(rest[..close].to_string());
        rest = &rest[close + 1..];
    }
    idents
}

fn is_command_ident(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('a'..='z'))
        && chars.all(|ch| matches!(ch, 'a'..='z' | '0'..='9' | '_'))
        && !value.contains('.')
}
