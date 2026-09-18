//! img:// 自定义协议：前端通过 convertFileSrc(path, "img") 加载封面与页面图片

use std::borrow::Cow;

use percent_encoding::percent_decode_str;
use tauri::http::{Request, Response, StatusCode};

/// 把本地图片文件以 http(s)://img.localhost/<url编码路径> 的形式提供给 WebView
pub fn img_protocol<R: tauri::Runtime>(
    _ctx: tauri::UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    match load_image(request.uri().path()) {
        Ok((bytes, mime)) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", mime)
            .body(bytes.into())
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(e.into_bytes().into())
            .unwrap(),
    }
}

fn load_image(encoded_path: &str) -> Result<(Vec<u8>, &'static str), String> {
    let decoded = percent_decode_str(encoded_path)
        .decode_utf8()
        .map_err(|e| format!("路径解码失败: {e}"))?;
    let mut path_str = decoded.as_ref();
    // Windows 盘符路径会被编码成 /D:/...，去掉开头的斜杠
    let mut chars = path_str.chars();
    if path_str.len() > 3 && path_str.starts_with('/') && chars.nth(1).map(|c| c.is_ascii_alphabetic() && path_str.as_bytes()[2] == b':').unwrap_or(false) {
        path_str = &path_str[1..];
    }
    let path = std::path::Path::new(path_str);
    if !path.is_file() {
        return Err(format!("文件不存在: {path_str}"));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("读取文件失败: {e}"))?;
    let mime = match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "application/octet-stream",
    };
    Ok((bytes, mime))
}
