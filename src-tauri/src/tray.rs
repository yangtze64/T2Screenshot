use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

pub fn create_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let capture_item = MenuItem::with_id(app, "capture", "截图", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    // 这两个权限菜单项仅 macOS 需要（热键需「辅助功能」、截图需「屏幕录制」）；
    // Windows 截图/热键无需此类授权，显示出来点了没反应反而困惑用户，故仅 macOS 创建。
    #[cfg(target_os = "macos")]
    let (perm_item, screen_item) = (
        MenuItem::with_id(app, "open_permissions", "打开快捷键权限设置…", true, None::<&str>)?,
        MenuItem::with_id(app, "open_screen_recording", "打开屏幕录制权限设置…", true, None::<&str>)?,
    );

    #[cfg(target_os = "macos")]
    let menu = Menu::with_items(app, &[&capture_item, &perm_item, &screen_item, &quit_item])?;
    #[cfg(not(target_os = "macos"))]
    let menu = Menu::with_items(app, &[&capture_item, &quit_item])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .menu(&menu)
        .tooltip("T2Screenshot")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "capture" => {
                super::start_screenshot(app);
            }
            "open_permissions" => {
                #[cfg(target_os = "macos")]
                {
                    // 跳到「辅助功能」隐私设置（全局热键所需）；必要时用户可手动切到「输入监控」
                    let _ = std::process::Command::new("open")
                        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                        .status();
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = app;
                }
            }
            "open_screen_recording" => {
                #[cfg(target_os = "macos")]
                {
                    // 跳到「屏幕录制」隐私设置：未授权时 macOS 截图只会截到桌面（其它窗口被隐藏）
                    let _ = std::process::Command::new("open")
                        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                        .status();
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = app;
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
