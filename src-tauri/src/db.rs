//! SQLite 数据访问层。数据库文件位于资源库的 `.comic-reader/comics.db`。

use std::path::Path;

use rusqlite::{params, Connection};

use crate::models::{ComicDto, ProgressDto};

pub const META_DIR: &str = ".comic-reader";
pub const COVERS_DIR: &str = "covers";
pub const CACHE_DIR: &str = "cache";
pub const DB_FILE: &str = "comics.db";

pub fn db_path(library: &Path) -> std::path::PathBuf {
    library.join(META_DIR).join(DB_FILE)
}

/// 打开（必要时创建）数据库并执行建表迁移
pub fn open(library: &Path) -> Result<Connection, String> {
    let path = db_path(library);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建数据库目录失败: {e}"))?;
    }
    let conn = Connection::open(path).map_err(|e| format!("打开数据库失败: {e}"))?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS comics (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            type TEXT NOT NULL,
            page_count INTEGER NOT NULL DEFAULT 0,
            added_at INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS reading_progress (
            comic_id TEXT PRIMARY KEY REFERENCES comics(id) ON DELETE CASCADE,
            page INTEGER NOT NULL DEFAULT 0,
            mode TEXT NOT NULL DEFAULT 'paged',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_comics_name ON comics(name);",
    )
    .map_err(|e| format!("初始化数据库表失败: {e}"))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 新增或更新漫画记录；返回该记录的 id
pub fn upsert(
    conn: &Connection,
    id: &str,
    name: &str,
    rel_path: &str,
    comic_type: &str,
    page_count: i64,
) -> Result<(), String> {
    let t = now();
    conn.execute(
        "INSERT INTO comics (id, name, path, type, page_count, added_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
         ON CONFLICT(path) DO UPDATE SET
            name = excluded.name,
            type = excluded.type,
            page_count = excluded.page_count,
            updated_at = excluded.updated_at",
        params![id, name, rel_path, comic_type, page_count, t],
    )
    .map_err(|e| format!("写入漫画记录失败: {e}"))?;
    Ok(())
}

/// 模糊搜索漫画（按名称），联表阅读进度
pub fn list_comics(conn: &Connection, query: &str) -> Result<Vec<ComicDto>, String> {
    let like = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, c.path, c.type, c.page_count,
                    p.page, p.mode
             FROM comics c
             LEFT JOIN reading_progress p ON p.comic_id = c.id
             WHERE c.name LIKE ?1 ESCAPE '\\'
             ORDER BY c.name COLLATE NOCASE",
        )
        .map_err(|e| format!("查询漫画失败: {e}"))?;
    let rows = stmt
        .query_map(params![like], |row| {
            Ok(ComicDto {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                comic_type: row.get(3)?,
                page_count: row.get(4)?,
                cover: None,
                progress_page: row.get(5)?,
                progress_mode: row.get(6)?,
            })
        })
        .map_err(|e| format!("读取漫画列表失败: {e}"))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("读取漫画记录失败: {e}"))?);
    }
    Ok(out)
}

/// 根据 id 取漫画的库内相对路径与类型
pub fn get_comic(conn: &Connection, id: &str) -> Result<Option<(String, String)>, String> {
    let mut stmt = conn
        .prepare("SELECT path, type FROM comics WHERE id = ?1")
        .map_err(|e| format!("查询漫画失败: {e}"))?;
    let mut rows = stmt
        .query_map(params![id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|e| format!("查询漫画失败: {e}"))?;
    Ok(rows.next().transpose().map_err(|e| format!("查询漫画失败: {e}"))?)
}

/// 所有漫画的 (id, path, page_count)，用于扫描时清理与增量更新
pub fn all_meta(conn: &Connection) -> Result<Vec<(String, String, i64)>, String> {
    let mut stmt = conn
        .prepare("SELECT id, path, page_count FROM comics")
        .map_err(|e| format!("查询漫画失败: {e}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|e| format!("查询漫画失败: {e}"))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| format!("读取漫画记录失败: {e}"))?);
    }
    Ok(out)
}

pub fn remove_comic(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM comics WHERE id = ?1", params![id])
        .map_err(|e| format!("删除漫画记录失败: {e}"))?;
    Ok(())
}

pub fn get_progress(conn: &Connection, id: &str) -> Result<Option<ProgressDto>, String> {
    let mut stmt = conn
        .prepare("SELECT page, mode FROM reading_progress WHERE comic_id = ?1")
        .map_err(|e| format!("查询阅读进度失败: {e}"))?;
    let mut rows = stmt
        .query_map(params![id], |row| {
            Ok(ProgressDto {
                page: row.get(0)?,
                mode: row.get(1)?,
            })
        })
        .map_err(|e| format!("查询阅读进度失败: {e}"))?;
    Ok(rows.next().transpose().map_err(|e| format!("查询阅读进度失败: {e}"))?)
}

pub fn save_progress(conn: &Connection, id: &str, page: i64, mode: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO reading_progress (comic_id, page, mode, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(comic_id) DO UPDATE SET
            page = excluded.page,
            mode = excluded.mode,
            updated_at = excluded.updated_at",
        params![id, page, mode, now()],
    )
    .map_err(|e| format!("保存阅读进度失败: {e}"))?;
    Ok(())
}
