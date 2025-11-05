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
