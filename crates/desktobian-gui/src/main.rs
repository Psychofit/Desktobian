//! Desktobian — wallpaper manager GUI (Tauri).
//!
//! A small desktop application to browse a library of wallpapers and apply one.
//! It does not render wallpapers itself: it drives the native renderer — the
//! KDE Plasma plugin via plasmashell D-Bus, or the standalone engine daemon via
//! the control socket (`desktobian-core::ipc`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod apply;
mod env;
mod library;

use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn get_environment() -> env::EnvInfo {
    env::detect()
}

#[tauri::command]
fn default_library_folders() -> Vec<String> {
    env::default_library_folders()
}

#[tauri::command]
fn scan_library(folders: Vec<String>, thumbnails: bool) -> Vec<library::WallpaperItem> {
    library::scan(&folders, thumbnails)
}

#[tauri::command]
fn apply_wallpaper(request: apply::ApplyRequest) -> apply::ApplyResult {
    apply::apply(request)
}

/// Native file picker for a single video/GIF.
#[tauri::command]
fn pick_video(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .add_filter(
            "Video & GIF",
            &[
                "mp4", "mkv", "webm", "mov", "avi", "m4v", "gif", "apng", "webp",
            ],
        )
        .blocking_pick_file()
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Native folder picker to add a library location.
#[tauri::command]
fn pick_folder(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_environment,
            default_library_folders,
            scan_library,
            apply_wallpaper,
            pick_video,
            pick_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Desktobian GUI");
}
