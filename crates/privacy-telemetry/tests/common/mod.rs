//! 测试用的日志捕获：把订阅器接到内存缓冲，不触碰全局状态，也不写入终端。

use privacy_telemetry::{Telemetry, TelemetryConfig};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Default)]
pub struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

impl SharedBuffer {
    pub fn contents(&self) -> String {
        let bytes = self.0.lock().expect("缓冲未被毒化").clone();
        String::from_utf8(bytes).expect("日志输出是 UTF-8")
    }
}

impl<'a> MakeWriter<'a> for SharedBuffer {
    type Writer = SharedWriter;

    fn make_writer(&'a self) -> Self::Writer {
        SharedWriter(self.0.clone())
    }
}

pub struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("缓冲未被毒化").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// 在给定配置下执行一段代码，返回它产生的全部日志文本。
pub fn capture(config: &TelemetryConfig, body: impl FnOnce()) -> String {
    let buffer = SharedBuffer::default();
    let subscriber = Telemetry::subscriber(config, buffer.clone()).expect("测试配置合法");
    tracing::subscriber::with_default(subscriber, body);
    buffer.contents()
}
