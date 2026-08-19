use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use std::sync::atomic::{AtomicU64, Ordering};

mod commands;
mod tray;

// 每次创建 overlay 用唯一 label，避免快速重复触发时同名窗口冲突（"already exists"）
static OVERLAY_SEQ: AtomicU64 = AtomicU64::new(0);

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
            commands::close_overlay,
            commands::windows::get_visible_windows,
        ])
        .setup(|app| {
            // 系统托盘
            let handle = app.handle().clone();
            tray::create_tray(&handle)?;

            // 主窗口点 ❌ 改为「隐藏」而非「销毁」：
            // 1) 保证截图结束后 show_main_window 总能恢复主窗口（否则窗口被销毁后无法恢复）；
            // 2) App 在仅剩托盘时不会退出（类似 QQ/微信截图的行为）。
            if let Some(main_win) = app.get_webview_window("main") {
                let app_handle = handle.clone();
                main_win.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = app_handle.get_webview_window("main") {
                            let _ = w.hide();
                        }
                    }
                });
            }

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
            eprintln!("[setup] global shortcut registered: {}", shortcut);
            #[cfg(target_os = "macos")]
            if !global_shortcut.is_registered(shortcut) {
                eprintln!(
                    "[setup] WARNING: 全局热键 '{}' 注册失败。macOS 通常需要在「系统设置 → 隐私与安全性 → 辅助功能」(必要时含「输入监控」) 中授权本应用，并重启 App。",
                    shortcut
                );
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn start_screenshot(app: &tauri::AppHandle) {
    // 先销毁所有已有的 overlay 窗口（可能残留上一次的），避免同名/竞态
    destroy_existing_overlays(app);

    // 先隐藏主窗口，避免冻结的截图里带上「截图」按钮窗口本身
    if let Some(main_win) = app.get_webview_window("main") {
        let _ = main_win.hide();
    }

    // 再截屏（此时主窗口已隐藏，截图为干净桌面）
    eprintln!("[start_screenshot] capturing screen...");
    match commands::capture::do_capture() {
        Ok(capture) => {
            eprintln!(
                "[start_screenshot] capture ok: work={}x{}, full={}x{}, scale={}",
                capture.width, capture.height, capture.full_width, capture.full_height, capture.scale_factor
            );

            // 使用完整屏幕尺寸（含程序坞）创建 overlay（先隐藏，图片就绪后再 show，避免闪屏）
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

fn destroy_existing_overlays(app: &tauri::AppHandle) {
    // 销毁所有 label 以 "overlay" 开头的窗口，避免快速重复触发时的竞态
    let labels: Vec<String> = app
        .webview_windows()
        .keys()
        .filter(|l| l.starts_with("overlay"))
        .map(|l| l.to_string())
        .collect();
    for label in labels {
        if let Some(w) = app.get_webview_window(&label) {
            let _ = w.destroy();
        }
    }
}

fn create_overlay_window(app: &tauri::AppHandle, width: f64, height: f64) {
    // 每次用唯一 label，彻底避免 "a webview with label `overlay` already exists"
    let label = format!("overlay-{}", OVERLAY_SEQ.fetch_add(1, Ordering::SeqCst));
    eprintln!("[create_overlay_window] label={} size={}x{}", label, width, height);
    match tauri::WebviewWindow::builder(
        app,
        label,
        tauri::WebviewUrl::App("/?overlay".into()),
    )
    .title("T2Screenshot Overlay")
    .inner_size(width, height)
    .position(0.0, 0.0)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(false)
    .transparent(true)
    .visible(true)
    .focused(true)
    .build()
    {
        Ok(w) => {
            eprintln!("[create_overlay_window] overlay window created successfully");
            // macOS：Tauri 的 always_on_top 对应 NSFloatingWindowLevel(=3)，
            // 而 Dock 为 20、菜单栏为 24，会把 overlay 盖住，导致 Dock 区域无法框选。
            // 这里把层级提到菜单栏之上，使整屏（含 Dock/菜单栏）都可被框选；
            // 同时关闭窗口动画与阴影，避免显示瞬间闪一下。
            #[cfg(target_os = "macos")]
            {
                let app_handle = app.clone();
                let _ = app_handle.run_on_main_thread(move || {
                    use objc2_app_kit::{
                        NSWindow, NSStatusWindowLevel, NSWindowAnimationBehavior,
                    };
                    if let Ok(ns_win) = w.ns_window() {
                        let ns_window: &NSWindow = unsafe { &*ns_win.cast() };
                        ns_window.setLevel(NSStatusWindowLevel + 1);
                        ns_window.setAnimationBehavior(NSWindowAnimationBehavior::None);
                        ns_window.setHasShadow(false);
                    }
                });
            }
        }
        Err(e) => {
            eprintln!("Failed to create overlay window: {}", e);
            if let Some(main_win) = app.get_webview_window("main") {
                let _ = main_win.show();
            }
        }
    }
}
