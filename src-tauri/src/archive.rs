//! zip 漫画包读取与解包

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::util::is_image_file;
use crate::util::natural_sort;

/// 列出 zip 内的图片条目名（自然排序）
pub fn list_images(path: &Path) -> Result<Vec<String>, String> {
    let file = File::open(path).map_err(|e| format!("打开压缩包失败 {}: {e}", path.display()))?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))
        .map_err(|e| format!("读取压缩包失败 {}: {e}", path.display()))?;
    let mut names = Vec::new();
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("读取压缩包条目失败: {e}"))?;
        let name = entry.name().to_string();
        if entry.is_file() && is_image_file(&name) {
            names.push(name);
        }
    }
    natural_sort(&mut names);
    Ok(names)
}

/// 读取 zip 内单个条目内容
pub fn read_entry(path: &Path, entry_name: &str) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("打开压缩包失败 {}: {e}", path.display()))?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))
        .map_err(|e| format!("读取压缩包失败 {}: {e}", path.display()))?;
    let mut entry = archive
        .by_name(entry_name)
        .map_err(|e| format!("压缩包中找不到条目 {entry_name}: {e}"))?;
    let mut buf = Vec::new();
    entry
        .read_to_end(&mut buf)
        .map_err(|e| format!("读取压缩包条目失败: {e}"))?;
    Ok(buf)
}

/// 把 zip 内所有图片解包到 out_dir，文件名为零填充序号（保持阅读顺序）。
/// 返回解出的图片数量。已存在的缓存目录会被清空重建。
pub fn extract_images(path: &Path, out_dir: &Path) -> Result<usize, String> {
    if out_dir.exists() {
        std::fs::remove_dir_all(out_dir).map_err(|e| format!("清理缓存失败: {e}"))?;
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("创建缓存目录失败: {e}"))?;

    let names = list_images(path)?;
    if names.is_empty() {
        return Err("压缩包内没有图片".to_string());
    }
    let file = File::open(path).map_err(|e| format!("打开压缩包失败 {}: {e}", path.display()))?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))
        .map_err(|e| format!("读取压缩包失败 {}: {e}", path.display()))?;

    let width = names.len().to_string().len().max(4);
    for (idx, name) in names.iter().enumerate() {
        let mut entry = archive
            .by_name(name)
            .map_err(|e| format!("压缩包中找不到条目 {name}: {e}"))?;
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("img")
            .to_lowercase();
        let dest = out_dir.join(format!("{:0width$}.{ext}", idx + 1, width = width));
        let mut out =
            File::create(&dest).map_err(|e| format!("创建缓存文件失败 {}: {e}", dest.display()))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("解包图片失败: {e}"))?;
    }
    Ok(names.len())
}
