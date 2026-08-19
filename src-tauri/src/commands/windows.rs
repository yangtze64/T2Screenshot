use serde::Serialize;
use xcap::Monitor;

#[derive(Serialize, Clone)]
pub struct WindowInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub app_name: String,
}

/// 获取所有可见窗口的列表（逻辑坐标）
#[tauri::command]
pub fn get_visible_windows() -> Result<Vec<WindowInfo>, String> {
    let monitor = Monitor::from_point(0, 0)
        .map_err(|e| format!("Failed to get monitor: {}", e))?;
    let scale_factor = monitor
        .scale_factor()
        .map_err(|e| format!("Failed to get scale factor: {}", e))? as f64;

    let windows = xcap::Window::all()
        .map_err(|e| format!("Failed to get windows: {}", e))?;

    let mut result = Vec::new();

    for win in windows {
        // 跳过无标题和极小的窗口
        let title = match win.title() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if title.is_empty() {
            continue;
        }

        let app_name = win.app_name().unwrap_or_default();

        let x = win.x().unwrap_or(0);
        let y = win.y().unwrap_or(0);
        let width = win.width().unwrap_or(0);
        let height = win.height().unwrap_or(0);

        // 跳过太小的窗口（如托盘图标等）
        if width < 50 || height < 50 {
            continue;
        }

        // 跳过自己的 overlay 窗口
        if title.contains("T2Screenshot") {
            continue;
        }

        // 转换为逻辑坐标
        result.push(WindowInfo {
            x: (x as f64 / scale_factor).round() as i32,
            y: (y as f64 / scale_factor).round() as i32,
            width: (width as f64 / scale_factor).round() as u32,
            height: (height as f64 / scale_factor).round() as u32,
            title,
            app_name,
        });
    }

    // 按窗口面积从大到小排序，小的窗口优先匹配（更精确）
    result.sort_by(|a, b| {
        (a.width as i64 * a.height as i64)
            .cmp(&(b.width as i64 * b.height as i64))
    });

    Ok(result)
}
