//! 二进制入口：安装日志、打开数据库、加载模型、装配路由并开始监听。

use clap::Parser;
mod bootstrap;
use bootstrap::Bootstrap;
use privacy_router::config::{Cli, Command, ServerArgs};
use privacy_router::server;
use privacy_router::state::AppState;
use privacy_store::Store;
use privacy_telemetry::{Telemetry, TelemetryConfig};
use std::process::ExitCode;
use std::sync::Arc;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let args = Arc::new(cli.server);

    // 日志必须最先安装，否则启动早期的失败无法被记录。
    let telemetry = TelemetryConfig {
        format: args.log_format.into(),
        filter: args.log_filter.clone(),
        log_dir: args.log_dir.clone(),
        ansi: None,
    };
    let guard = match Telemetry::init(&telemetry) {
        Ok(guard) => guard,
        Err(error) => {
            eprintln!("无法安装日志订阅器：{error}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!(error = %error, "无法创建异步运行时");
            return ExitCode::FAILURE;
        }
    };

    let outcome = match cli.command {
        None => runtime.block_on(serve(args)),
        Some(Command::Model { action }) => runtime.block_on(Bootstrap::new(args).model(action)),
        Some(Command::InitAdmin {
            username,
            password,
            force,
        }) => runtime.block_on(init_admin(args, username, password, force)),
    };

    match outcome {
        Ok(()) => {
            // 先冲刷日志再退出，避免最后若干条记录丢失。
            drop(guard);
            ExitCode::SUCCESS
        }
        Err(message) => {
            tracing::error!(stage = "startup", error = %message, "启动失败");
            eprintln!("{message}");
            drop(guard);
            ExitCode::FAILURE
        }
    }
}

async fn serve(args: Arc<ServerArgs>) -> Result<(), String> {
    args.validate()?;
    if !args.is_loopback() && !args.allow_public_bind {
        return Err(format!(
            "拒绝绑定到 {}：控制台没有内置传输层加密。\n\
             确认要在非回环地址上暴露后，重试并加上 --allow-public-bind。",
            args.bind
        ));
    }

    let bootstrap = Bootstrap::new(args.clone());
    bootstrap.prepare().await?;
    bootstrap.stage("Opening audit database and loading rules");

    let store = Arc::new(
        Store::open(&args.database)
            .await
            .map_err(|error| format!("无法打开数据库 {}：{error}", args.database.display()))?,
    );

    // 默认规则只在数据库生命周期内安装一次；之后完全服从管理员改动。
    let builtin = privacy_rules::RuleSet::builtin();
    let seeded = store
        .rules()
        .seed_builtin(builtin.rules())
        .await
        .map_err(|error| format!("无法写入内置规则：{error}"))?;
    if seeded > 0 {
        tracing::info!(stage = "startup", seeded, "默认规则已安装");
    }

    let rules = store
        .rules()
        .load_set()
        .await
        .map_err(|error| format!("无法装配规则集合：{error}"))?;

    let classifier = bootstrap.load().await?;
    bootstrap.stage("Starting HTTP server");
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(
            args.upstream_timeout_seconds,
        ))
        .build()
        .map_err(|error| format!("无法创建上游 HTTP 客户端：{error}"))?;

    let state = Arc::new(AppState::new(
        store.clone(),
        classifier,
        http,
        args.clone(),
        rules,
    ));
    let app = server::router(state);

    let listener = tokio::net::TcpListener::bind(args.bind)
        .await
        .map_err(|error| format!("无法监听 {}：{error}", args.bind))?;

    bootstrap.finish();
    if !args.is_loopback() {
        tracing::warn!(
            bind = %args.bind,
            "控制台已绑定到非回环地址，任何能访问该端口的人都可以查看审计内容"
        );
    }
    tracing::info!(bind = %args.bind, database = %args.database.display(), "代理已启动");

    // 连接来源必须可见：首次初始化的自助入口只对本机开放。
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .map_err(|error| format!("服务异常结束：{error}"))
}

async fn init_admin(
    args: Arc<ServerArgs>,
    username: String,
    password: Option<String>,
    force: bool,
) -> Result<(), String> {
    let store = Store::open(&args.database)
        .await
        .map_err(|error| format!("无法打开数据库：{error}"))?;

    let password = match password {
        Some(password) => password,
        // 口令默认从终端读取，避免出现在 shell 历史与进程列表里。
        None => rpassword::prompt_password("管理员口令：")
            .map_err(|error| format!("无法读取口令：{error}"))?,
    };
    // 凭据是否可用由 store 的构造校验裁决，这里不再重复表达同一件事。
    let credentials = privacy_store::Credentials::new(username, password)
        .map_err(|error| format!("无法使用该凭据：{error}"))?;

    if store
        .admin()
        .is_configured()
        .await
        .map_err(|error| format!("无法检查管理员状态：{error}"))?
    {
        if !force {
            return Err("管理员已存在；如需重置凭据请加上 --force（这会使全部会话失效)".to_owned());
        }
        store
            .admin()
            .replace_credentials(&credentials)
            .await
            .map_err(|error| format!("无法重置凭据：{error}"))?;
        println!("凭据已重置，全部既有会话已失效。");
        return Ok(());
    }

    store
        .admin()
        .create(&credentials)
        .await
        .map_err(|error| format!("无法创建管理员：{error}"))?;
    println!("管理员 {} 已创建。", credentials.username());
    Ok(())
}

/// Ctrl-C 与 SIGTERM 都触发优雅退出，让正在进行的请求有机会完成。
async fn shutdown_signal() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            signal.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = interrupt => {}
        _ = terminate => {}
    }
    tracing::info!(stage = "shutdown", "收到退出信号，正在停止");
}
