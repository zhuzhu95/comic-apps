//! 封面生成与 PDF 页面渲染

use std::path::{Path, PathBuf};

use image::DynamicImage;
use mupdf::{Colorspace, Document, ImageFormat, Matrix};

use crate::archive;
use crate::models::CoverSource;

/// 封面缩略图最长边
const COVER_MAX: u32 = 400;
/// 阅读器页面渲染目标宽度（PDF）
const PAGE_TARGET_WIDTH: f32 = 1400.0;

/// 生成封面缩略图到 covers_dir/<id>.jpg
pub fn generate(covers_dir: &Path, id: &str, source: &CoverSource) -> Result<(), String> {
    let img = load_source_image(source)?;
    save_thumbnail(&img, &cover_path(covers_dir, id))
}

pub fn cover_path(covers_dir: &Path, id: &str) -> PathBuf {
    covers_dir.join(format!("{id}.jpg"))
}

fn save_thumbnail(img: &DynamicImage, dest: &Path) -> Result<(), String> {
    let thumb = img.thumbnail(COVER_MAX, COVER_MAX);
    thumb
        .to_rgb8()
        .save_with_format(dest, image::ImageFormat::Jpeg)
        .map_err(|e| format!("保存封面失败 {}: {e}", dest.display()))
}

fn load_source_image(source: &CoverSource) -> Result<DynamicImage, String> {
    match source {
        CoverSource::Image(path) => image::open(path)
            .map_err(|e| format!("读取图片失败 {}: {e}", path.display())),
        CoverSource::ZipEntry(zip, entry) => {
            // 先按内存解码，失败则解出临时文件再解码（兼容个别格式）
            let bytes = archive::read_entry(zip, entry)?;
            match image::load_from_memory(&bytes) {
                Ok(img) => Ok(img),
                Err(_) => {
                    let tmp = std::env::temp_dir().join(format!(
                        "comic-cover-{}-{}",
                        std::process::id(),
                        sanitize_file_name(entry)
                    ));
                    std::fs::write(&tmp, &bytes).map_err(|e| format!("写入临时文件失败: {e}"))?;
                    let result = image::open(&tmp);
                    let _ = std::fs::remove_file(&tmp);
                    result.map_err(|e| format!("解码压缩包内图片失败 {entry}: {e}"))
                }
            }
        }
        CoverSource::Pdf(path) => {
            let doc = Document::open(path.to_string_lossy().as_ref())
                .map_err(|e| format!("打开 PDF 失败 {}: {e}", path.display()))?;
            render_page(&doc, 0, Some(COVER_MAX as f32))
        }
    }
}

/// 渲染文档某一页为图片；target_width 用于计算缩放倍数（None 表示原始大小）
pub fn render_page(doc: &Document, page_no: i32, target_width: Option<f32>) -> Result<DynamicImage, String> {
    let page = doc
        .load_page(page_no)
        .map_err(|e| format!("加载 PDF 第 {} 页失败: {e}", page_no + 1))?;
    let bounds = page.bounds().map_err(|e| format!("读取 PDF 页面尺寸失败: {e}"))?;
    let zoom = match target_width {
        Some(w) if bounds.width() > 0.0 => w / bounds.width(),
        _ => 1.0,
    };
    let pixmap = page
        .to_pixmap(
            &Matrix::new_scale(zoom, zoom),
            &Colorspace::device_rgb(),
            false,
            false,
        )
        .map_err(|e| format!("渲染 PDF 第 {} 页失败: {e}", page_no + 1))?;
    let mut buf = Vec::new();
    pixmap
        .write_to(&mut buf, ImageFormat::PNM)
        .map_err(|e| format!("导出 PDF 页面图像失败: {e}"))?;
    image::load_from_memory(&buf).map_err(|e| format!("解析 PDF 页面图像失败: {e}"))
}

/// 渲染 PDF 全部页面为 jpg 到 out_dir（文件名零填充序号），返回页数。
/// 缓存目录已存在时直接复用。
pub fn render_pdf_pages(pdf: &Path, out_dir: &Path) -> Result<usize, String> {
    if out_dir.exists() && std::fs::read_dir(out_dir).map(|mut d| d.next().is_some()).unwrap_or(false) {
        let count = std::fs::read_dir(out_dir)
            .map_err(|e| format!("读取缓存目录失败: {e}"))?
            .filter_map(|e| e.ok())
            .filter(|e| is_image_file(&e.file_name().to_string_lossy()))
            .count();
        if count > 0 {
            return Ok(count);
        }
    }
    if out_dir.exists() {
        std::fs::remove_dir_all(out_dir).map_err(|e| format!("清理缓存失败: {e}"))?;
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("创建缓存目录失败: {e}"))?;

    let doc = Document::open(pdf.to_string_lossy().as_ref())
        .map_err(|e| format!("打开 PDF 失败 {}: {e}", pdf.display()))?;
    let page_count = doc
        .page_count()
        .map_err(|e| format!("读取 PDF 页数失败: {e}"))?;
    if page_count <= 0 {
        return Err("PDF 没有页面".to_string());
    }
    let width = (page_count as usize).to_string().len().max(4);
    for i in 0..page_count {
        let img = render_page(&doc, i, Some(PAGE_TARGET_WIDTH))?;
        let dest = out_dir.join(format!("{:0width$}.jpg", i + 1, width = width));
        img.to_rgb8()
            .save_with_format(&dest, image::ImageFormat::Jpeg)
            .map_err(|e| format!("保存页面失败 {}: {e}", dest.display()))?;
    }
    Ok(page_count as usize)
}

fn is_image_file(name: &str) -> bool {
    crate::util::is_image_file(name)
}

fn sanitize_file_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect()
}
