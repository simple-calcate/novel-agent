//! Tauri command 按领域分模块：library（结构/章节）、editor（编辑会话）、
//! settings（模型配置/偏好）、plugins、sync（outbox）、queue、canon（正史/设定）。

pub mod canon;
pub mod editor;
pub mod library;
pub mod plugins;
pub mod queue;
pub mod settings;
pub mod sync;
