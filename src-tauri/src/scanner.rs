//! 资源库扫描：识别漫画条目、生成封面、同步数据库

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use mupdf::Document;
use rusqlite::Connection;
use uuid::Uuid;

use crate::cover;
use crate::db;
use crate::models::{ComicType, CoverSource, ScannedComic};
use crate::util::{is_image_file, natural_sort};

/// 扫描资源库：发现漫画、清理失效条目、为新增/变更的漫画生成封面
pub fn scan(library: &Path, conn: &Connection) -> Result<(), String> {
    let meta_dir = library.join(db::META_DIR);
    let covers_dir = meta_dir.join(db::COVERS_DIR);
    let cache_dir = meta_dir.join(db::CACHE_DIR);
    fs::create_dir_all(&covers_dir).map_err(|e| format!("创建封面目录失败: {e}"))?;
    fs::create_dir_all(&cache_dir).map_err(|e| format!("创建缓存目录失败: {e}"))?;

    let mut scanned = Vec::new();
    let entries = fs::read_dir(library).map_err(|e| format!("读取资源库目录失败: {e}"))?;
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        // 跳过隐藏项与元数据目录
        if name.starts_with('.') || name == db::META_DIR {
            continue;
        }
        let path = entry.path();
        if let Some(comic) = if path.is_dir() {
            scan_dir(library, &path)
        } else {
            scan_file(library, &path)
        } {
            scanned.push(comic);
        }
    }

    // 清理数据库中已不存在的漫画
    let existing = db::all_meta(conn)?;
    let found_paths: HashSet<&str> = scanned.iter().map(|c| c.rel_path.as_str()).collect();
    for (id, path, _) in &existing {
        if !found_paths.contains(path.as_str()) {
            db::remove_comic(conn, id)?;
            let _ = fs::remove_file(cover::cover_path(&covers_dir, id));
            let _ = fs::remove_dir_all(cache_dir.join(id));
        }
    }
    let meta_by_path: std::collections::HashMap<String, (String, i64)> = existing
        .into_iter()
        .map(|(id, path, pc)| (path, (id, pc)))
        .collect();

    // 入库并为新漫画生成封面
    for comic in &scanned {
        let (id, old_page_count) = meta_by_path
            .get(&comic.rel_path)
            .cloned()
            .unwrap_or_else(|| (Uuid::new_v4().to_string(), -1));
        db::upsert(
            conn,
            &id,
            &comic.name,
            &comic.rel_path,
            comic.comic_type.as_str(),
            comic.page_count,
        )?;
        let cover_file = cover::cover_path(&covers_dir, &id);
        if !cover_file.exists() || old_page_count != comic.page_count {
            if let Err(e) = cover::generate(&covers_dir, &id, &comic.cover) {
                eprintln!("[scanner] 生成封面失败 {}: {e}", comic.rel_path);
            }
        }
    }
    Ok(())
}

/// 判定一个文件夹是否为漫画资源：
/// 1. 顶层有图片 → 散图漫画
/// 2. 否则取与文件夹同名的 zip（或目录内唯一的 zip）→ zip 漫画
fn scan_dir(library: &Path, dir: &Path) -> Option<ScannedComic> {
    let dir_name = dir.file_name()?.to_string_lossy().to_string();

    let mut images: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && is_image_file(&crate::util::file_name_string(p)))
        .collect();
    if !images.is_empty() {
        images.sort_by(|a, b| {
            crate::util::natural_cmp(
                &crate::util::file_name_string(a),
                &crate::util::file_name_string(b),
            )
        });
        let page_count = images.len() as i64;
        return Some(ScannedComic {
            name: dir_name.clone(),
            rel_path: rel_path(library, dir),
            comic_type: ComicType::Images,
            page_count,
            cover: CoverSource::Image(images[0].clone()),
        });
    }

    // 查找 zip：优先与目录同名，其次目录内唯一的一个 zip
    let zips: Vec<PathBuf> = fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file() && p.extension().map(|e| e.to_string_lossy().to_lowercase()) == Some("zip".to_string())
        })
        .collect();
    let zip = zips
        .iter()
        .find(|p| p.file_stem().map(|s| s.to_string_lossy()) == Some(dir_name.as_str().into()))
        .or_else(|| if zips.len() == 1 { zips.first() } else { None })?;
    scan_zip(library, zip, &dir_name)
}

/// 判定单个文件是否为漫画资源（zip / pdf）
fn scan_file(library: &Path, file: &Path) -> Option<ScannedComic> {
    let ext = file.extension()?.to_string_lossy().to_lowercase();
    let name = file.file_stem()?.to_string_lossy().to_string();
    match ext.as_str() {
        "zip" => scan_zip(library, file, &name),
        "pdf" => {
            let page_count = Document::open(file.to_string_lossy().as_ref())
                .and_then(|d| d.page_count())
                .unwrap_or(0) as i64;
            if page_count <= 0 {
                return None;
            }
            Some(ScannedComic {
                name,
                rel_path: rel_path(library, file),
                comic_type: ComicType::Pdf,
                page_count,
                cover: CoverSource::Pdf(file.to_path_buf()),
            })
        }
        _ => None,
    }
}

fn scan_zip(library: &Path, zip: &Path, name: &str) -> Option<ScannedComic> {
    let images = crate::archive::list_images(zip).ok()?;
    if images.is_empty() {
        return None;
    }
    Some(ScannedComic {
        name: name.to_string(),
        rel_path: rel_path(library, zip),
        comic_type: ComicType::Zip,
        page_count: images.len() as i64,
        cover: CoverSource::ZipEntry(zip.to_path_buf(), images[0].clone()),
    })
}

fn rel_path(library: &Path, path: &Path) -> String {
    path.strip_prefix(library)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

/// 收集文件夹（不递归）内排序后的图片绝对路径
pub fn list_dir_images(dir: &Path) -> Vec<PathBuf> {
    let mut images: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && is_image_file(&crate::util::file_name_string(p)))
            .collect(),
        Err(_) => Vec::new(),
    };
    let mut names: Vec<String> = images
        .iter()
        .map(|p| crate::util::file_name_string(p))
        .collect();
    natural_sort(&mut names);
    let order: std::collections::HashMap<String, usize> = names
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, n)| (n, i))
        .collect();
    images.sort_by_key(|p| order[&crate::util::file_name_string(p)]);
    images
}
