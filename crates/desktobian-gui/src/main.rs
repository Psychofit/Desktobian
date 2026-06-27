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

/// Scanning (and thumbnailing via ffmpeg) can be slow, so run it on the blocking
/// pool rather than the main thread.
#[tauri::command]
async fn scan_library(folders: Vec<String>, thumbnails: bool) -> Vec<library::WallpaperItem> {
    tauri::async_runtime::spawn_blocking(move || library::scan(&folders, thumbnails))
        .await
        .unwrap_or_default()
}

/// Applying may shell out to qdbus/the engine socket; keep it off the main thread.
#[tauri::command]
async fn apply_wallpaper(request: apply::ApplyRequest) -> apply::ApplyResult {
    tauri::async_runtime::spawn_blocking(move || apply::apply(request))
        .await
        .unwrap_or_else(|e| apply::ApplyResult {
            ok: false,
            message: format!("internal error: {e}"),
            method: String::new(),
        })
}

/// Native file picker for a single video/GIF.
///
/// This is an `async` command on purpose: synchronous Tauri commands run on the
/// main thread, and the *blocking* dialog APIs would then deadlock the GTK event
/// loop. As an async command it runs off the main thread, so we use the
/// non-blocking picker (which schedules the dialog on the main loop) and await
/// its result over a oneshot channel.
#[tauri::command]
async fn pick_video(app: tauri::AppHandle) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter(
            "Video & GIF",
            &[
                "mp4", "mkv", "webm", "mov", "avi", "m4v", "gif", "apng", "webp",
            ],
        )
        .pick_file(move |f| {
            let _ = tx.send(f);
        });
    let picked = rx.await.ok().flatten()?;
    picked
        .into_path()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Native folder picker to add a library location.
#[tauri::command]
async fn pick_folder(app: tauri::AppHandle) -> Option<String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |f| {
        let _ = tx.send(f);
    });
    let picked = rx.await.ok().flatten()?;
    picked
        .into_path()
        .ok()
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
