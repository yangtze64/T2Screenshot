use base64::Engine;
use serde::Serialize;
use std::sync::Mutex;
use xcap::Monitor;

#[derive(Serialize, Clone)]
pub struct ScreenCaptureResult {
    pub image_base64: String,
    /// 工作区逻辑宽度（不含程序坞）
    pub width: u32,
    /// 工作区逻辑高度（不含程序坞）
    pub height: u32,
    /// 完整屏幕逻辑宽度（含程序坞）
    pub full_width: u32,
    /// 完整屏幕逻辑高度（含程序坞）
    pub full_height: u32,
    pub scale_factor: f64,
}

/// 全局缓存：截图后暂存，供 overlay 窗口拉取
pub static PENDING_CAPTURE: Mutex<Option<ScreenCaptureResult>> = Mutex::new(None);

#[tauri::command]
pub fn capture_screen() -> Result<ScreenCaptureResult, String> {
    do_capture_internal()
}

/// 获取 Mutex 锁，自动恢复 poison 锁
fn get_lock() -> std::sync::MutexGuard<'static, Option<ScreenCaptureResult>> {
    PENDING_CAPTURE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// overlay 窗口挂载后调用，拉取截图数据
#[tauri::command]
pub fn get_pending_capture() -> Result<ScreenCaptureResult, String> {
    let mut pending = get_lock();
    match pending.take() {
        Some(result) => Ok(result),
        None => Err("No pending capture".into()),
    }
}

pub fn do_capture() -> Result<ScreenCaptureResult, String> {
    let result = do_capture_internal()?;
    // 缓存到全局
    let mut pending = get_lock();
    *pending = Some(result.clone());
    Ok(result)
}

fn do_capture_internal() -> Result<ScreenCaptureResult, String> {
    let monitor = Monitor::from_point(0, 0).map_err(|e| format!("Failed to get monitor: {}", e))?;
    let width = monitor.width().map_err(|e| format!("Failed to get width: {}", e))?;
    let height = monitor.height().map_err(|e| format!("Failed to get height: {}", e))?;

    let image = monitor.capture_image().map_err(|e| format!("Failed to capture: {}", e))?;

    // 通过图片实际像素与逻辑像素的比值计算 scale_factor
    let scale_factor = image.width() as f64 / width as f64;

    // 获取包含程序坞在内的完整屏幕尺寸
    let (full_width, full_height) = get_full_screen_size(scale_factor);

    let mut png_data = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png_data), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    let engine = base64::engine::general_purpose::STANDARD;
    let image_base64 = engine.encode(&png_data);

    Ok(ScreenCaptureResult {
        image_base64,
        width,
        height,
        full_width,
        full_height,
        scale_factor,
    })
}

/// 获取包含程序坞在内的完整屏幕逻辑尺寸
#[cfg(target_os = "macos")]
fn get_full_screen_size(scale_factor: f64) -> (u32, u32) {
    use core_graphics::display::{CGMainDisplayID, CGDisplayBounds};
    unsafe {
        let display_id = CGMainDisplayID();
        let bounds = CGDisplayBounds(display_id);
        let w = (bounds.size.width as u32).max(1);
        let h = (bounds.size.height as u32).max(1);
        eprintln!(
            "[get_full_screen_size] CGDisplayBounds: {}x{}, scale={}",
            w, h, scale_factor
        );
        (w, h)
    }
}

#[cfg(not(target_os = "macos"))]
fn get_full_screen_size(scale_factor: f64) -> (u32, u32) {
    // 非 macOS 平台使用 xcap 的工作区尺寸
    match Monitor::from_point(0, 0) {
        Ok(monitor) => {
            let w = monitor.width().unwrap_or(1920);
            let h = monitor.height().unwrap_or(1080);
            (w, h)
        }
        Err(_) => (1920, 1080),
    }
}
