//! 漫画数据模型（前后端传输用 DTO）

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ComicDto {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub comic_type: String,
    pub path: String,
    pub page_count: i64,
    /// 封面缩略图的绝对路径，前端通过 img:// 协议加载；无封面为 None
    pub cover: Option<String>,
    pub progress_page: Option<i64>,
    pub progress_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProgressDto {
    pub page: i64,
    pub mode: String,
}

/// 漫画资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComicType {
    /// 文件夹内散图
    Images,
    /// zip 压缩包
    Zip,
    /// pdf
    Pdf,
}

impl ComicType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ComicType::Images => "images",
            ComicType::Zip => "zip",
            ComicType::Pdf => "pdf",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "images" => Some(ComicType::Images),
            "zip" => Some(ComicType::Zip),
            "pdf" => Some(ComicType::Pdf),
            _ => None,
        }
    }
}

/// 封面来源（扫描时确定，生成封面用）
#[derive(Debug, Clone)]
pub enum CoverSource {
    /// 磁盘图片文件
    Image(std::path::PathBuf),
    /// zip 包 + 内部条目名
    ZipEntry(std::path::PathBuf, String),
    /// pdf 文件（取第一页）
    Pdf(std::path::PathBuf),
}

/// 扫描发现的单部漫画
#[derive(Debug, Clone)]
pub struct ScannedComic {
    pub name: String,
    /// 相对资源库根的路径（文件夹名或文件名）
    pub rel_path: String,
    pub comic_type: ComicType,
    pub page_count: i64,
    pub cover: CoverSource,
}
