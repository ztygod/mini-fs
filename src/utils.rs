use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 生成一个随机唯一 ID
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// 格式化时间戳为可读字符串
pub fn format_time(ts: u64) -> String {
    use chrono::{DateTime, Local};
    use std::time::{Duration, UNIX_EPOCH};

    let dt = UNIX_EPOCH + Duration::from_secs(ts);
    let datetime: DateTime<Local> = dt.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn split_path(path: &str) -> Result<(&str, &str), String> {
    let path = path.trim();

    if path.is_empty() {
        return Err("Empty path".into());
    }

    // 去掉结尾的 '/'
    let path = path.trim_end_matches('/');

    // 根目录不能 create
    if path == "/" {
        return Err("Cannot create root".into());
    }

    match path.rfind('/') {
        Some(0) => {
            // "/file"
            Ok(("/", &path[1..]))
        }
        Some(pos) => {
            // "/a/b/file"
            Ok((&path[..pos], &path[pos + 1..]))
        }
        None => {
            // "file"（相对路径，视你 FS 设计）
            Ok(("/", path))
        }
    }
}
