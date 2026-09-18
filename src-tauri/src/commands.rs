//! Tauri 命令实现

use std::fs;
use std::path::{Path, PathBuf};

use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use rusqlite::Connection;
use tauri::{AppHandle, Manager, State};

use crate::cover;
use crate::db;
use crate::models::{ComicDto, ComicType, ProgressDto};
use crate::scanner;

pub struct AppState {
    pub library: Mutex<Option<PathBuf>>,
    pub db: Mutex<Option<Connection>>,
}

impl AppState {
    fn library(&self) -> Result<PathBuf, String> {
        self.library
            .lock()
            .clone()
            .ok_or_else(|| "尚未设置资源库文件夹".to_string())
    }

    fn conn(&self) -> Result<MappedMutexGuard<'_, Connection>, String> {
        let guard = self.db.lock();
        if guard.is_none() {
            return Err("尚未设置资源库文件夹".to_string());
        }
        Ok(MutexGuard::map(guard, |g| g.as_mut().unwrap()))
    }
}

// ---------- 设置 ----------

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用数据目录失败: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    Ok(dir.join("settings.json"))
}

fn read_settings(app: &AppHandle) -> Option<serde_json::Value> {
    let content = fs::read_to_string(settings_path(app).ok()?).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_settings(app: &AppHandle, library: &Path) -> Result<(), String> {
    let json = serde_json::json!({ "library": library });
    fs::write(
        settings_path(app)?,
        serde_json::to_string_pretty(&json).map_err(|e| format!("序列化设置失败: {e}"))?,
    )
    .map_err(|e| format!("保存设置失败: {e}"))
}

/// 启动时恢复上次的资源库设置
pub fn init_from_settings(app: &mut tauri::App) {
    let Some(settings) = read_settings(&app.handle()) else {
        return;
    };
    let Some(path) = settings.get("library").and_then(|v| v.as_str()) else {
        return;
    };
    let path = PathBuf::from(path);
    if !path.is_dir() {
        return;
    }
    if let Ok(conn) = db::open(&path) {
        let state = app.state::<AppState>();
        *state.library.lock() = Some(path);
        *state.db.lock() = Some(conn);
    }
}

/// 获取当前资源库路径
#[tauri::command]
pub fn get_library(state: State<'_, AppState>) -> Option<String> {
    state.library.lock().clone().map(|p| p.to_string_lossy().to_string())
}

/// 设置资源库文件夹并执行首次扫描
#[tauri::command]
pub fn set_library(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<ComicDto>, String> {
    let path = PathBuf::from(path);
    if !path.is_dir() {
        return Err("所选路径不是文件夹".to_string());
    }
    let conn = db::open(&path)?;
    write_settings(&app, &path)?;
    *state.library.lock() = Some(path.clone());
    *state.db.lock() = Some(conn);
    scan_library(state)
}

// ---------- 库查询 ----------

/// 列出漫画（按名称模糊搜索）
#[tauri::command]
pub fn list_comics(state: State<'_, AppState>, query: String) -> Result<Vec<ComicDto>, String> {
    let conn = state.conn()?;
    let library = state.library()?;
    let covers_dir = library.join(db::META_DIR).join(db::COVERS_DIR);
    let mut comics = db::list_comics(&conn, &query)?;
    for comic in &mut comics {
        let cover_file = cover::cover_path(&covers_dir, &comic.id);
        if cover_file.exists() {
            comic.cover = Some(cover_file.to_string_lossy().to_string());
        }
    }
    Ok(comics)
}

/// 重新扫描资源库
#[tauri::command]
pub fn scan_library(state: State<'_, AppState>) -> Result<Vec<ComicDto>, String> {
    {
        let conn = state.conn()?;
        let library = state.library()?;
        scanner::scan(&library, &conn)?;
    }
    list_comics(state, String::new())
}

// ---------- 导入 ----------

/// 把外部文件/文件夹复制进资源库，完成后扫描
#[tauri::command]
pub fn import_paths(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<ComicDto>, String> {
    let library = state.library()?;
    for p in &paths {
        let src = PathBuf::from(p);
        if !src.exists() {
            return Err(format!("路径不存在: {p}"));
        }
        import_one(&library, &src)?;
    }
    scan_library(state)
}

fn import_one(library: &Path, src: &Path) -> Result<(), String> {
    let stem = src
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| format!("无效路径: {}", src.display()))?;
    let dest = unique_dest(library, &stem, if src.is_dir() { None } else { src.extension().and_then(|e| e.to_str()) });
    if src.is_dir() {
        copy_dir(src, &dest)?;
    } else {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }
        fs::copy(src, &dest).map_err(|e| format!("复制文件失败 {}: {e}", src.display()))?;
    }
    Ok(())
}

/// 目标已存在时追加序号：name (2).zip
fn unique_dest(dir: &Path, stem: &str, ext: Option<&str>) -> PathBuf {
    for i in 0.. {
        let name = if i == 0 {
            stem.to_string()
        } else {
            format!("{stem} ({})", i + 1)
        };
        let file_name = match ext {
            Some(e) => format!("{name}.{e}"),
            None => name,
        };
        let candidate = dir.join(&file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!()
}

fn copy_dir(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| format!("创建目录失败: {e}"))?;
    for entry in walkdir::WalkDir::new(src).min_depth(1) {
        let entry = entry.map_err(|e| format!("遍历目录失败: {e}"))?;
        let rel = entry
            .path()
            .strip_prefix(src)
            .map_err(|e| format!("计算相对路径失败: {e}"))?;
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|e| format!("创建目录失败: {e}"))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
            }
            fs::copy(entry.path(), &target)
                .map_err(|e| format!("复制文件失败 {}: {e}", entry.path().display()))?;
        }
    }
    Ok(())
}

// ---------- 阅读 ----------

/// 获取漫画页面（绝对路径列表，前端经 img:// 协议加载）
#[tauri::command]
pub fn get_comic_pages(state: State<'_, AppState>, id: String) -> Result<Vec<String>, String> {
    let conn = state.conn()?;
    let library = state.library()?;
    let (rel_path, comic_type) = db::get_comic(&conn, &id)?
        .ok_or_else(|| "漫画不存在".to_string())?;
    let comic_path = library.join(&rel_path);
    let comic_type = ComicType::from_str(&comic_type)
        .ok_or_else(|| format!("未知漫画类型: {comic_type}"))?;

    let cache_dir = library.join(db::META_DIR).join(db::CACHE_DIR).join(&id);
    let pages: Vec<PathBuf> = match comic_type {
        ComicType::Images => {
            if !comic_path.is_dir() {
                return Err(format!("漫画文件夹不存在: {}", comic_path.display()));
            }
            scanner::list_dir_images(&comic_path)
        }
        ComicType::Zip => {
            crate::archive::extract_images(&comic_path, &cache_dir)?;
            scanner::list_dir_images(&cache_dir)
        }
        ComicType::Pdf => {
            cover::render_pdf_pages(&comic_path, &cache_dir)?;
            scanner::list_dir_images(&cache_dir)
        }
    };
    if pages.is_empty() {
        return Err("没有找到页面".to_string());
    }
    Ok(pages.into_iter().map(|p| p.to_string_lossy().to_string()).collect())
}

#[tauri::command]
pub fn get_progress(state: State<'_, AppState>, id: String) -> Result<Option<ProgressDto>, String> {
    let conn = state.conn()?;
    db::get_progress(&conn, &id)
}

#[tauri::command]
pub fn save_progress(
    state: State<'_, AppState>,
    id: String,
    page: i64,
    mode: String,
) -> Result<(), String> {
    let conn = state.conn()?;
    db::save_progress(&conn, &id, page, &mode)
}

// ---------- 删除 ----------

/// 删除漫画：总是删除数据库记录、封面与缓存；remove_files 为 true 时同时删除源文件
#[tauri::command]
pub fn delete_comic(
    state: State<'_, AppState>,
    id: String,
    remove_files: bool,
) -> Result<Vec<ComicDto>, String> {
    let conn = state.conn()?;
    let library = state.library()?;
    if let Some((rel_path, _)) = db::get_comic(&conn, &id)? {
        let comic_path = library.join(&rel_path);
        if remove_files {
            if comic_path.is_dir() {
                fs::remove_dir_all(&comic_path)
                    .map_err(|e| format!("删除文件夹失败: {e}"))?;
            } else if comic_path.exists() {
                fs::remove_file(&comic_path).map_err(|e| format!("删除文件失败: {e}"))?;
            }
        }
    }
    db::remove_comic(&conn, &id)?;
    let _ = fs::remove_file(cover::cover_path(
        &library.join(db::META_DIR).join(db::COVERS_DIR),
        &id,
    ));
    let _ = fs::remove_dir_all(library.join(db::META_DIR).join(db::CACHE_DIR).join(&id));
    drop(conn);
    list_comics(state, String::new())
}
