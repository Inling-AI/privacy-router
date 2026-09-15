//! 枚举与 TEXT 之间的转换。
//!
//! 存储层不在数据库里维护第二份字符串表：映射统一经过 serde 派生的名称，`CHECK` 约束
//! 使用的字面量与这里的序列化结果必须一致，由集成测试断言。

use crate::{Error, Result};
use serde::{Serialize, de::DeserializeOwned};

/// 把派生为字符串的枚举写入 TEXT 列。
pub(crate) fn encode<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| Error::Encoding("值必须序列化为 JSON 字符串".into()))
}

/// 从 TEXT 列恢复派生为字符串的枚举。
pub(crate) fn decode<T: DeserializeOwned>(text: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(text.to_owned()))
        .map_err(|error| Error::Encoding(error.to_string()))
}
