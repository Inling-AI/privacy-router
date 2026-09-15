use crate::{Error, Result};

/// 单次设备执行的上限；输入较多时自动拆成多个微批次。
/// 采用无 padding 的打包形式，token 预算是有效 token 总数。
#[derive(Debug, Clone, Copy)]
pub struct BatchLimits {
    pub max_sequences: usize,
    pub max_tokens: usize,
}

impl Default for BatchLimits {
    fn default() -> Self {
        Self {
            max_sequences: 16,
            max_tokens: 4096,
        }
    }
}

impl BatchLimits {
    /// 校验微批次预算，零预算返回参数错误。
    pub fn validate(self) -> Result<()> {
        if self.max_sequences == 0 || self.max_tokens == 0 {
            return Err(Error::InvalidOptions(
                "batch limits must be positive".into(),
            ));
        }
        Ok(())
    }
}
