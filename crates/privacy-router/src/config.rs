//! 运行配置。所有可调项都来自命令行或环境变量，没有隐式默认之外的第三处来源。

use clap::{Parser, Subcommand, ValueEnum};
use privacy_filter_cache::CacheLimits;
use privacy_telemetry::LogFormat;
use std::net::SocketAddr;
use std::path::PathBuf;

/// 代理与控制台。
#[derive(Debug, Parser)]
#[command(name = "privacy-router", version, about)]
pub struct Cli {
    #[command(flatten)]
    pub server: ServerArgs,

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Runtime inference selection. CPU remains available in GPU-enabled binaries.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum InferenceBackend {
    #[default]
    Cpu,
    #[cfg(feature = "gpu")]
    Gpu,
}

#[derive(Debug, clap::Args)]
pub struct ServerArgs {
    /// Inference backend. CPU works without GPU drivers; GPU requires a compatible device.
    #[arg(long, env = "PRIVACY_ROUTER_BACKEND", value_enum, default_value_t = InferenceBackend::Cpu)]
    pub backend: InferenceBackend,

    /// 监听地址。默认只绑定回环，控制台不对外暴露。
    #[arg(long, env = "PRIVACY_ROUTER_BIND", default_value = "127.0.0.1:8787")]
    pub bind: SocketAddr,

    /// SQLite 数据库文件。
    #[arg(
        long,
        env = "PRIVACY_ROUTER_DATABASE",
        default_value = "privacy-router.db"
    )]
    pub database: PathBuf,

    /// 模型目录，需包含 config.json、tokenizer.json、model.safetensors。
    #[arg(
        long,
        env = "PRIVACY_ROUTER_MODEL_DIR",
        default_value = privacy_model::DEFAULT_DIRECTORY
    )]
    pub model_dir: PathBuf,

    /// Pin the download source (huggingface or modelscope); by default benchmark all sources first.
    #[arg(long, env = "PRIVACY_ROUTER_MODEL_SOURCE")]
    pub model_source: Option<privacy_model::Hub>,

    /// Verify local model files without contacting download providers.
    #[arg(long, env = "PRIVACY_ROUTER_OFFLINE")]
    pub offline: bool,

    /// 日志格式。
    #[arg(long, env = "PRIVACY_ROUTER_LOG_FORMAT", value_enum, default_value_t = LogStyle::Human)]
    pub log_format: LogStyle,

    /// 日志过滤指令；默认读取 RUST_LOG。
    #[arg(long, env = "PRIVACY_ROUTER_LOG_FILTER", default_value = "info")]
    pub log_filter: String,

    /// 同时写入该目录下的滚动日志文件。
    #[arg(long, env = "PRIVACY_ROUTER_LOG_DIR")]
    pub log_dir: Option<PathBuf>,

    /// 请求体上限（字节）。超过即拒绝，不转发。
    #[arg(long, env = "PRIVACY_ROUTER_MAX_BODY_BYTES", default_value_t = 32 * 1024 * 1024)]
    pub max_body_bytes: usize,

    /// 单次送入模型的 token 上限。
    #[arg(long, env = "PRIVACY_ROUTER_MAX_TOKENS", default_value_t = 8192)]
    pub max_tokens: usize,

    /// 结果缓存条目上限。
    #[arg(long, env = "PRIVACY_ROUTER_CACHE_ENTRIES", default_value_t = 4096)]
    pub cache_entries: usize,

    /// 结果缓存保留数据上限（字节）。
    #[arg(long, env = "PRIVACY_ROUTER_CACHE_BYTES", default_value_t = 16 * 1024 * 1024)]
    pub cache_bytes: usize,

    /// 控制台会话有效期（秒）。
    #[arg(long, env = "PRIVACY_ROUTER_SESSION_TTL", default_value_t = 12 * 60 * 60)]
    pub session_ttl_seconds: i64,

    /// 上游请求超时（秒）。
    #[arg(long, env = "PRIVACY_ROUTER_UPSTREAM_TIMEOUT", default_value_t = 600)]
    pub upstream_timeout_seconds: u64,

    /// 绑定到非回环地址时必须显式确认；否则拒绝启动。
    #[arg(
        long,
        env = "PRIVACY_ROUTER_ALLOW_PUBLIC_BIND",
        default_value_t = false
    )]
    pub allow_public_bind: bool,

    /// 开发期允许的控制台来源，可重复传入。
    ///
    /// 生产形态下控制台与本进程同源提供，永远不需要跨源；只有 `flutter run` 那种
    /// 独立开发服务器才需要。留空即完全不挂 CORS 层，代理端点 `/v1/*` 在任何情况下
    /// 都不会获得跨源许可。
    #[arg(long, env = "PRIVACY_ROUTER_DEV_ORIGIN", value_delimiter = ',')]
    pub dev_origin: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LogStyle {
    Human,
    Json,
}

impl From<LogStyle> for LogFormat {
    fn from(style: LogStyle) -> Self {
        match style {
            LogStyle::Human => Self::Human,
            LogStyle::Json => Self::Json,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Prepare or verify model files without starting the server.
    Model {
        #[command(subcommand)]
        action: ModelAction,
    },
    /// 直接创建管理员账号；首次安装也可以直接在浏览器里自助初始化。
    /// 账号已存在时必须显式确认才覆盖凭据。
    InitAdmin {
        #[arg(long, env = "PRIVACY_ROUTER_ADMIN_USER", default_value = "admin")]
        username: String,
        /// 口令。省略时从标准输入读取，避免出现在进程列表里。
        #[arg(long, env = "PRIVACY_ROUTER_ADMIN_PASSWORD")]
        password: Option<String>,
        #[arg(long, default_value_t = false)]
        force: bool,
    },
}

#[derive(Debug, Clone, Copy, Subcommand)]
pub enum ModelAction {
    /// Benchmark providers, then download and verify missing artifacts.
    Download,
    /// Check all local artifact checksums without network access.
    Verify,
    /// Measure all selected providers without downloading model files.
    Benchmark,
}

impl ServerArgs {
    /// 拒绝自相矛盾的组合。开发来源会把控制台 API 的跨源许可交给浏览器，因此
    /// 只在回环绑定下成立。
    pub fn validate(&self) -> Result<(), String> {
        if !self.dev_origin.is_empty() && !self.is_loopback() {
            return Err(format!(
                "--dev-origin 只能配合回环绑定使用，当前绑定为 {}",
                self.bind
            ));
        }
        Ok(())
    }

    pub fn cache_limits(&self) -> CacheLimits {
        CacheLimits {
            max_entries: self.cache_entries,
            max_bytes: self.cache_bytes,
        }
    }

    pub fn batch_limits(&self) -> privacy_filter::BatchLimits {
        privacy_filter::BatchLimits {
            max_sequences: 16,
            max_tokens: self.max_tokens,
        }
    }

    pub fn session_ttl_ms(&self) -> i64 {
        self.session_ttl_seconds.saturating_mul(1000)
    }

    /// 绑定地址是否只对本机可见。
    pub fn is_loopback(&self) -> bool {
        self.bind.ip().is_loopback()
    }
}
