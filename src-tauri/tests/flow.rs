//! 后端功能集成测试：以 /tmp/comic-test-lib 为资源库（由 /tmp/make-test-data.py 生成），
//! 覆盖扫描、封面生成、zip 解析、PDF 渲染、搜索与阅读进度。

use std::path::Path;

use comic_apps_lib::{archive, cover, db, scanner};

const TEST_LIB: &str = "/tmp/comic-test-lib";

fn ensure_test_lib() -> bool {
    Path::new(TEST_LIB).is_dir()
}

#[test]
fn test_full_flow() {
    if !ensure_test_lib() {
        eprintln!("测试资源库不存在，跳过: {TEST_LIB}");
        return;
    }
    let lib = Path::new(TEST_LIB);
    let conn = db::open(lib).expect("打开数据库");

    // 1. 扫描：应识别出 4 部漫画
    scanner::scan(lib, &conn).expect("扫描资源库");
    let comics = db::list_comics(&conn, "").expect("列出漫画");
    assert_eq!(comics.len(), 4, "应扫描出 4 部漫画: {comics:?}");
    for c in &comics {
        assert!(c.page_count > 0, "{} 页数应大于 0", c.name);
        // 2. 封面应已生成
        let cover_file = lib.join(db::META_DIR).join(db::COVERS_DIR).join(format!("{}.jpg", c.id));
        assert!(cover_file.is_file(), "{} 应生成封面文件", c.name);
    }

    // 各类型与页数校验
    let by_name = |name: &str| comics.iter().find(|c| c.name == name).cloned().unwrap();
    let images_comic = by_name("散图漫画-01");
    assert_eq!(images_comic.comic_type, "images");
    assert_eq!(images_comic.page_count, 4);
    let zip_comic = by_name("ZIP漫画-02");
    assert_eq!(zip_comic.comic_type, "zip");
    assert_eq!(zip_comic.page_count, 3);
    let dir_zip = by_name("文件夹ZIP-03");
    assert_eq!(dir_zip.comic_type, "zip");
    assert_eq!(dir_zip.page_count, 2);
    let pdf_comic = by_name("PDF漫画-04");
    assert_eq!(pdf_comic.comic_type, "pdf");
    assert_eq!(pdf_comic.page_count, 3);

    // 3. 自然排序：散图漫画第一页应是 1.png（而不是 10.png）
    let images_dir = lib.join("散图漫画-01");
    let pages = scanner::list_dir_images(&images_dir);
    assert_eq!(
        pages[0].file_name().unwrap().to_string_lossy(),
        "1.png",
        "散图应自然排序"
    );

    // 4. zip 条目自然排序 + 解包
    let zip_path = lib.join("ZIP漫画-02.zip");
    let entries = archive::list_images(&zip_path).expect("列出 zip 图片");
    assert_eq!(entries.len(), 3);
    let out = std::env::temp_dir().join("comic-test-extract");
    let n = archive::extract_images(&zip_path, &out).expect("解包 zip");
    assert_eq!(n, 3);
    let extracted = scanner::list_dir_images(&out);
    assert_eq!(extracted.len(), 3);
    assert!(extracted[0].file_name().unwrap().to_string_lossy().starts_with("0001"));
    std::fs::remove_dir_all(&out).ok();

    // 5. PDF 渲染全部页面
    let pdf_out = std::env::temp_dir().join("comic-test-pdf");
    let count = cover::render_pdf_pages(&lib.join("PDF漫画-04.pdf"), &pdf_out).expect("渲染 PDF");
    assert_eq!(count, 3);
    assert_eq!(scanner::list_dir_images(&pdf_out).len(), 3);
    std::fs::remove_dir_all(&pdf_out).ok();

    // 6. 模糊搜索
    let hits = db::list_comics(&conn, "ZIP").expect("搜索");
    assert_eq!(hits.len(), 2, "搜索 ZIP 应命中 2 部");
    let miss = db::list_comics(&conn, "不存在的名字").expect("搜索");
    assert!(miss.is_empty());

    // 7. 阅读进度存取
    db::save_progress(&conn, &images_comic.id, 2, "scroll").expect("保存进度");
    let progress = db::get_progress(&conn, &images_comic.id).expect("读取进度").unwrap();
    assert_eq!(progress.page, 2);
    assert_eq!(progress.mode, "scroll");

    // 8. 重复扫描应保持稳定（数量不变，不产生重复）
    scanner::scan(lib, &conn).expect("重复扫描");
    let comics2 = db::list_comics(&conn, "").expect("再次列出");
    assert_eq!(comics2.len(), 4);
    let progress2 = db::get_progress(&conn, &images_comic.id).expect("进度仍在").unwrap();
    assert_eq!(progress2.page, 2, "重复扫描不应丢失阅读进度");
}
