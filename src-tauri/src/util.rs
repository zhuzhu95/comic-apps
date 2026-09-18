//! 通用工具：图片扩展名判断、文件名自然排序

use std::path::Path;

/// 支持的图片扩展名（小写、不带点）
pub const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

pub fn is_image_file(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    IMAGE_EXTS.contains(&ext.as_str())
}

/// 自然排序比较：`2` < `10`，`a2` < `a10`
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    // 收集完整数字段（去掉前导零后比较，长度优先）
                    let na = take_digits(&mut ai);
                    let nb = take_digits(&mut bi);
                    let sa = na.trim_start_matches('0');
                    let sb = nb.trim_start_matches('0');
                    let ord = sa
                        .len()
                        .cmp(&sb.len())
                        .then_with(|| sa.cmp(sb))
                        .then_with(|| na.len().cmp(&nb.len()));
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                } else {
                    let la: String = ca.to_lowercase().collect();
                    let lb: String = cb.to_lowercase().collect();
                    let ord = la.cmp(&lb);
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                    ai.next();
                    bi.next();
                }
            }
        }
    }
}

fn take_digits(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    while let Some(&c) = it.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        s.push(c);
        it.next();
    }
    s
}

pub fn natural_sort(names: &mut [String]) {
    names.sort_by(|a, b| natural_cmp(a, b));
}

/// 文件名（不含路径），异常路径返回空串
pub fn file_name_string(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natural_sort() {
        let mut v = vec![
            "10.png".to_string(),
            "2.png".to_string(),
            "1.png".to_string(),
            "a10.png".to_string(),
            "a2.png".to_string(),
        ];
        natural_sort(&mut v);
        assert_eq!(
            v,
            vec!["1.png", "2.png", "10.png", "a2.png", "a10.png"]
        );
    }
}
