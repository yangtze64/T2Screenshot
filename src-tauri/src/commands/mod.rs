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

/// 关闭 overlay：恢复主窗口，并由 Rust 侧销毁 overlay 窗口。
///
/// 为什么不在前端用 `appWindow.destroy()` 销毁当前窗口：
/// 在窗口自身 JS 上下文里调用 `destroy()`，窗口销毁时会切断 IPC 通道，
/// 导致该 Promise 永不 resolve（表现为「点击复制/取消毫无反应」）。
/// 改为由 Rust 在命令返回响应之后再销毁，可靠且不会卡死前端。
#[tauri::command]
pub fn close_overlay(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.show();
    }

    // 稍候再销毁，确保本命令的 IPC 响应先发回前端；销毁必须在主线程执行。
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(80));
        let app_inner = app2;
        let _ = app_inner.run_on_main_thread({
            let app_clone = app_inner.clone();
            move || {
                let labels: Vec<String> = app_clone
                    .webview_windows()
                    .keys()
                    .filter(|l| l.starts_with("overlay"))
                    .cloned()
                    .collect();
                eprintln!("[close_overlay] destroying overlay windows: {:?}", labels);
                for label in labels {
                    if let Some(w) = app_clone.get_webview_window(&label) {
                        let _ = w.destroy();
                    }
                }
            }
        });
    });

    Ok(())
}
