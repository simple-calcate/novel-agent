//! Tauri 命令层： AppState 与应用装配。
//! 具体命令按领域拆在 `commands/`，共享管道在 `common.rs`。

mod common;
pub mod commands;

// 扁平再导出：invoke_handler 与 command_tests 均按旧路径引用
pub use common::CommandResult;
pub use commands::canon::*;
pub use commands::editor::*;
pub use commands::library::*;
pub use commands::plugins::*;
pub use commands::queue::*;
pub use commands::settings::*;
pub use commands::sync::*;

use novel_automation::TypingSession;
use novel_extensions::{BuiltinsExtension, SecretVault};
use novel_kernel::Kernel;
use novel_storage::StorageHandle;
use std::sync::{Arc, Mutex};
use tauri::Manager;

pub struct AppState {
    pub kernel: Arc<Kernel>,
    pub typing_session: Mutex<TypingSession>,
}

#[cfg(test)]
mod command_tests;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::DEBUG.into()),
        )
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let database = data_dir.join("novel-agent.sqlite3");
            let storage = Arc::new(StorageHandle::open(database)?);
            let secrets = Arc::new(SecretVault::open(&data_dir));

            let kernel = Kernel::builder()
                .service(storage)
                .service(secrets)
                .extension(BuiltinsExtension)?
                .build()?;
            app.manage(AppState {
                kernel: Arc::new(kernel),
                typing_session: Mutex::new(TypingSession::new(chrono::Utc::now())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_project,
            create_book,
            create_volume,
            create_chapter,
            create_scene,
            rename_scene,
            set_scene_pov,
            delete_scene,
            move_scene,
            load_library,
            set_active_project,
            load_chapter,
            save_chapter,
            list_chapter_revisions,
            diff_chapter_revisions,
            restore_chapter_revision,
            rename_project,
            delete_project,
            rename_book,
            delete_book,
            move_book,
            rename_volume,
            delete_volume,
            move_volume,
            rename_chapter,
            delete_chapter,
            move_chapter,
            editor_tick,
            context_hints,
            save_model_config,
            load_model_config,
            generate_continuation,
            install_plugin_manifest,
            commit_annotation,
            emit_domain_event,
            emit_block_mode_changed,
            build_context_package,
            run_queue_step,
            kernel_tools,
            enqueue_job,
            list_jobs,
            propose_canon,
            list_canon,
            review_canon_fact,
            create_story_entry,
            list_story_entries,
            delete_story_entry,
            record_generation_feedback,
            list_preferences,
            set_preference_status,
            list_plugins,
            run_plugin_operation,
            pending_outbox_count,
            flush_outbox_journal,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("novel agent 启动失败: {err}");
            std::process::exit(1);
        });
}
