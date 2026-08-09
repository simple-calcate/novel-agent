use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedBook {
    pub title: String,
    pub synopsis: String,
    pub chapters: Vec<ExportedChapter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedChapter {
    pub title: String,
    pub position: u32,
    pub revision: u64,
    pub status: String,
    pub content: String,
    pub scenes: Vec<ExportedScene>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedScene {
    pub title: String,
    pub position: u32,
    pub content: String,
}

pub fn export_book_txt(book: &ExportedBook) -> String {
    let mut out = String::new();
    out.push_str(&book.title);
    out.push_str("\n\n");
    if !book.synopsis.is_empty() {
        out.push_str(&format!("{}\n\n", book.synopsis));
    }
    for chapter in &book.chapters {
        out.push_str(&format!("第{}章 {}\n\n", chapter.position, chapter.title));
        out.push_str(&chapter.content);
        out.push_str("\n\n");
    }
    out
}

pub fn export_book_markdown(book: &ExportedBook) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {}\n\n", book.title));
    if !book.synopsis.is_empty() {
        out.push_str(&format!("> {}\n\n", book.synopsis));
    }
    for chapter in &book.chapters {
        out.push_str(&format!("## 第{}章 {}\n\n", chapter.position, chapter.title));
        out.push_str(&chapter.content);
        out.push_str("\n\n");
    }
    out
}

pub fn parse_txt_import(title: &str, content: &str) -> ExportedBook {
    let mut chapters = Vec::new();
    let mut current_title = String::from("正文");
    let mut current_lines = Vec::new();
    let mut position = 1u32;

    for line in content.lines() {
        if line.starts_with("第") && line.contains("章") {
            if !current_lines.is_empty() {
                chapters.push(ExportedChapter {
                    title: current_title.clone(),
                    position,
                    revision: 0,
                    status: "draft".into(),
                    content: current_lines.join("\n"),
                    scenes: vec![],
                });
                position += 1;
                current_lines.clear();
            }
            current_title = line.trim().to_owned();
        } else {
            current_lines.push(line);
        }
    }

    if !current_lines.is_empty() {
        chapters.push(ExportedChapter {
            title: current_title,
            position,
            revision: 0,
            status: "draft".into(),
            content: current_lines.join("\n"),
            scenes: vec![],
        });
    }

    ExportedBook {
        title: title.to_owned(),
        synopsis: String::new(),
        chapters,
    }
}
