use std::io::{self, Write};
use tracing_subscriber::fmt::MakeWriter;

/// 把同一份日志同时送往两个 sink，使「写终端」与「写文件」共用同一个格式化层。
#[derive(Debug, Clone, Copy)]
pub struct Tee<A, B> {
    first: A,
    second: B,
}

impl<A, B> Tee<A, B> {
    pub fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

impl<'a, A, B> MakeWriter<'a> for Tee<A, B>
where
    A: MakeWriter<'a>,
    B: MakeWriter<'a>,
{
    type Writer = TeeWriter<A::Writer, B::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        TeeWriter {
            first: self.first.make_writer(),
            second: self.second.make_writer(),
        }
    }
}

/// 单条日志的写入目标；两侧都写完才返回。
pub struct TeeWriter<A, B> {
    first: A,
    second: B,
}

impl<A: Write, B: Write> Write for TeeWriter<A, B> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.first.write(buf)?;
        // 一侧失败不应让另一侧收到截断的记录。
        self.second.write_all(&buf[..written])?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.first.flush()?;
        self.second.flush()
    }
}
