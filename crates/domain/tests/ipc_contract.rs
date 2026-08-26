use novel_domain::{Book, Chapter, Project, Scene, StoryEntry, Volume};

const EXAMPLES: &str = include_str!("../../../packages/shared-types/examples.json");

#[test]
fn ipc_examples_deserialize_into_domain() {
    let value: serde_json::Value = serde_json::from_str(EXAMPLES).unwrap();
    let _: Project = serde_json::from_value(value["project"].clone()).unwrap();
    let _: Book = serde_json::from_value(value["book"].clone()).unwrap();
    let _: Volume = serde_json::from_value(value["volume"].clone()).unwrap();
    let _: Chapter = serde_json::from_value(value["chapter"].clone()).unwrap();
    let _: Scene = serde_json::from_value(value["scene"].clone()).unwrap();
    let _: StoryEntry = serde_json::from_value(value["storyEntry"].clone()).unwrap();
}
