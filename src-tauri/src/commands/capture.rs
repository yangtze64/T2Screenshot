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

    // 计算 scale_factor：
    // - macOS：monitor.width() 返回逻辑像素，image 是物理像素，二者比值 = 真实 scale（Retina=2.0），可行；
    // - Windows：monitor.width() 返回的是 EnumDisplaySettings 的物理像素，与 image 同为物理像素，
    //   比值恒为 1.0，会丢失 DPI 缩放信息 → 坐标换算错位、overlay 窗口尺寸错误。
    // 因此统一优先用 xcap 的 monitor.scale_factor()（权威 DPI 缩放），失败时再回退到比值。
    let scale_factor = monitor
        .scale_factor()
        .map(|s| s as f64)
        .unwrap_or_else(|_| image.width() as f64 / width as f64);
    eprintln!(
        "[do_capture] image={}x{}, monitor.wh={}x{}, scale_factor={}",
        image.width(),
        image.height(),
        width,
        height,
        scale_factor
    );

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
    // Windows/Linux：monitor.width()/height() 返回物理像素，
    // 而 Tauri 的 WebviewWindow::inner_size 期望逻辑像素。
    // 若直接传物理像素，在 DPI 缩放下 overlay 窗口会比屏幕大（如 150% 时大 1.5 倍），
    // 多出部分跑到屏幕外 → 用户看到「主窗口消失、overlay 也不可见」的假死现象。
    // 故这里除以 scale_factor 换算成逻辑像素。
    match Monitor::from_point(0, 0) {
        Ok(monitor) => {
            let pw = monitor.width().unwrap_or(1920) as f64;
            let ph = monitor.height().unwrap_or(1080) as f64;
            let sf = if scale_factor > 0.0 { scale_factor } else { 1.0 };
            let lw = ((pw / sf).round() as u32).max(1);
            let lh = ((ph / sf).round() as u32).max(1);
            eprintln!(
                "[get_full_screen_size] physical={}x{}, scale={}, logical={}x{}",
                pw, ph, sf, lw, lh
            );
            (lw, lh)
        }
        Err(_) => (1920, 1080),
    }
}
