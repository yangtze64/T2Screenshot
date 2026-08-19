use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

mod commands;
mod tray;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::capture::capture_screen,
            commands::capture::get_pending_capture,
            commands::save::save_screenshot,
            commands::save::save_screenshot_with_dialog,
            commands::save::copy_to_clipboard,
            commands::trigger_screenshot,
            commands::show_main_window,
            commands::windows::get_visible_windows,
        ])
        .setup(|app| {
            // 系统托盘
            let handle = app.handle().clone();
            tray::create_tray(&handle)?;

            // 注册全局热键 (macOS: Cmd+Shift+A, Windows: Alt+Shift+A)
            let shortcut = if cfg!(target_os = "macos") {
                "Cmd+Shift+A"
            } else {
                "Alt+Shift+A"
            };

            let global_shortcut = app.global_shortcut();
            global_shortcut.on_shortcut(shortcut, |app, _shortcut, _event| {
                start_screenshot(app);
            })?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn start_screenshot(app: &tauri::AppHandle) {
    // 先销毁已有的 overlay 窗口
    if let Some(overlay_win) = app.get_webview_window("overlay") {
        let _ = overlay_win.destroy();
    }

    // 先截屏（此时其他应用窗口仍可见，主窗口也会在截图中但不影响使用）
    eprintln!("[start_screenshot] capturing screen...");
    match commands::capture::do_capture() {
        Ok(capture) => {
            eprintln!(
                "[start_screenshot] capture ok: work={}x{}, full={}x{}, scale={}",
                capture.width, capture.height, capture.full_width, capture.full_height, capture.scale_factor
            );

            // 截屏成功后，隐藏主窗口
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.hide();
            }

            // 使用完整屏幕尺寸（含程序坞）创建 overlay
            let logical_w = capture.full_width as f64;
            let logical_h = capture.full_height as f64;

            create_overlay_window(app, logical_w, logical_h);
        }
        Err(e) => {
            eprintln!("[start_screenshot] capture failed: {}", e);
            // 截图失败，恢复主窗口
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.show();
            }
        }
    }
}

fn create_overlay_window(app: &tauri::AppHandle, width: f64, height: f64) {
    eprintln!("[create_overlay_window] size={}x{}", width, height);
    match tauri::WebviewWindow::builder(
        app,
        "overlay",
        tauri::WebviewUrl::App("/?overlay".into()),
    )
    .title("T2Screenshot Overlay")
    .inner_size(width, height)
    .position(0.0, 0.0)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(true)
    .focused(true)
    .build()
    {
        Ok(_w) => {
            eprintln!("[create_overlay_window] overlay window created successfully");
        }
        Err(e) => {
            eprintln!("Failed to create overlay window: {}", e);
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.show();
            }
        }
    }
}
