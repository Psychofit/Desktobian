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

/// The editable properties of a web wallpaper, read from the `project.json`
/// next to its entry HTML. Empty for non-web wallpapers or when there's no
/// readable project.json — the UI then just shows no property editor.
#[tauri::command]
fn web_properties(path: String) -> Vec<desktobian_core::webprops::WebProperty> {
    let entry = std::path::Path::new(&path);
    let Some(dir) = entry.parent() else {
        return Vec::new();
    };
    match std::fs::read_to_string(dir.join("project.json")) {
        Ok(text) => desktobian_core::webprops::parse_properties(&text),
        Err(_) => Vec::new(),
    }
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
            web_properties,
            apply_wallpaper,
            pick_video,
            pick_folder,
        ])
        .setup(setup_tray)
        // Closing the window hides it to the tray (the wallpaper keeps playing);
        // the app only really quits via the tray's "Quit" entry.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running the Desktobian GUI");
}

/// The wallpaper that was set before Desktobian touched it, so we can put it
/// back on quit. Managed as Tauri state.
struct OriginalWallpaper(std::sync::Mutex<Option<apply::SavedWallpaper>>);

/// Build the system tray icon and its menu, and snapshot the current wallpaper.
fn setup_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::Manager;

    // Remember the wallpaper that was there at launch so "Quit" can restore it.
    app.manage(OriginalWallpaper(std::sync::Mutex::new(apply::capture())));

    let show = MenuItem::with_id(app, "show", "Show Desktobian", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit (restore wallpaper)", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let mut builder = TrayIconBuilder::new()
        .tooltip("Desktobian")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                // Restore the original wallpaper (or fall back to a default),
                // then quit.
                let original = app
                    .state::<OriginalWallpaper>()
                    .0
                    .lock()
                    .ok()
                    .and_then(|g| g.clone());
                match original {
                    Some(saved) => apply::restore(&saved),
                    None => apply::revert_to_default(),
                }
                app.exit(0);
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
