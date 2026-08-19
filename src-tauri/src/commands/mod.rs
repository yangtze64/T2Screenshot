pub mod capture;
pub mod save;
pub mod windows;

use tauri::Manager;

#[tauri::command]
pub fn trigger_screenshot(app: tauri::AppHandle) -> Result<(), String> {
    crate::start_screenshot(&app);
    Ok(())
}

/// 显示主窗口（overlay 关闭时调用）
#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.show();
    }
    Ok(())
}
