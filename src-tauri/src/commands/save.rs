use base64::Engine;
use std::path::PathBuf;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;

/// 裁剪参数
#[derive(serde::Deserialize, Debug)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// 从原始全屏截图 base64 中裁剪指定区域，返回裁剪后的 base64
fn crop_base64(image_base64: &str, crop: &CropRect) -> Result<String, String> {
    let engine = base64::engine::general_purpose::STANDARD;
    let png_data = engine.decode(image_base64).map_err(|e| format!("Failed to decode base64: {}", e))?;

    let mut img = image::load_from_memory(&png_data).map_err(|e| format!("Failed to load image: {}", e))?;

    // 边界检查
    let (img_w, img_h) = (img.width(), img.height());
    if crop.x + crop.width > img_w || crop.y + crop.height > img_h {
        return Err(format!(
            "Crop region ({},{},{},{}) out of bounds ({},{})",
            crop.x, crop.y, crop.width, crop.height, img_w, img_h
        ));
    }

    let cropped = img.crop(crop.x, crop.y, crop.width, crop.height);

    let mut buf = Vec::new();
    cropped
        .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    Ok(engine.encode(&buf))
}

#[tauri::command]
pub fn save_screenshot(
    app: tauri::AppHandle,
    image_base64: String,
    crop: Option<CropRect>,
    path: Option<String>,
) -> Result<String, String> {
    eprintln!("[save_screenshot] called, base64_len={}, crop={:?}, path={:?}", image_base64.len(), crop, path);
    let final_base64 = match crop {
        Some(ref c) => crop_base64(&image_base64, c)?,
        None => image_base64.clone(),
    };

    let engine = base64::engine::general_purpose::STANDARD;
    let png_data = engine.decode(&final_base64).map_err(|e| format!("Failed to decode base64: {}", e))?;

    let save_path = match path {
        Some(p) => {
            let sp = PathBuf::from(&p);
            if let Some(parent) = sp.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
            }
            sp
        }
        None => {
            let dir = dirs::picture_dir()
                .or_else(dirs::home_dir)
                .unwrap_or_else(|| PathBuf::from("."));
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            dir.join(format!("screenshot_{}.png", timestamp))
        }
    };

    std::fs::write(&save_path, &png_data).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(save_path.to_string_lossy().to_string())
}

/// 弹出保存对话框并保存截图（异步，因为需要等待用户选择路径）
#[tauri::command]
pub async fn save_screenshot_with_dialog(
    app: tauri::AppHandle,
    image_base64: String,
    crop: Option<CropRect>,
) -> Result<String, String> {
    eprintln!("[save_screenshot_with_dialog] called, base64_len={}, crop={:?}", image_base64.len(), crop);

    // 弹出保存文件对话框
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let default_name = format!("screenshot_{}.png", timestamp);

    let file_path = app.dialog()
        .file()
        .add_filter("PNG", &["png"])
        .set_file_name(&default_name)
        .blocking_save_file()
        .map(|p| p.to_string())
        .unwrap_or_default();

    eprintln!("[save_screenshot_with_dialog] dialog returned path: {:?}", file_path);

    if file_path.is_empty() {
        return Err("用户取消了保存".into());
    }

    // 调用已有的保存逻辑
    save_screenshot(app, image_base64, crop, Some(file_path))
}

#[tauri::command]
pub fn copy_to_clipboard(
    app: tauri::AppHandle,
    image_base64: String,
    crop: Option<CropRect>,
) -> Result<(), String> {
    eprintln!("[copy_to_clipboard] called, base64_len={}, crop={:?}", image_base64.len(), crop);
    let final_base64 = match crop {
        Some(ref c) => crop_base64(&image_base64, c)?,
        None => image_base64.clone(),
    };

    let engine = base64::engine::general_purpose::STANDARD;
    let png_data = engine.decode(&final_base64).map_err(|e| format!("Failed to decode base64: {}", e))?;

    let img = image::load_from_memory(&png_data).map_err(|e| format!("Failed to load image: {}", e))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let rgba_data = rgba.into_raw();

    let tauri_img = tauri::image::Image::new_owned(rgba_data, width, height);

    app.clipboard()
        .write_image(&tauri_img)
        .map_err(|e| format!("Failed to write to clipboard: {}", e))?;

    eprintln!(
        "[copy_to_clipboard] success: wrote {}x{} image to clipboard",
        width, height
    );

    Ok(())
}
