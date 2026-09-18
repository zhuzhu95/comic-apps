//! 漫画阅读器 Tauri 后端入口

pub mod archive;
pub mod commands;
pub mod cover;
pub mod db;
pub mod models;
pub mod protocol;
pub mod scanner;
pub mod util;

use parking_lot::Mutex;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            library: Mutex::new(None),
            db: Mutex::new(None),
        })
        .setup(|app| {
            commands::init_from_settings(app);
            Ok(())
        })
        .register_uri_scheme_protocol("img", protocol::img_protocol)
        .invoke_handler(tauri::generate_handler![
            commands::get_library,
            commands::set_library,
            commands::list_comics,
            commands::scan_library,
            commands::import_paths,
            commands::get_comic_pages,
            commands::get_progress,
            commands::save_progress,
            commands::delete_comic,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
