use std::time::Duration;

use anyhow::Result;
use axum::{routing::get, Router};
use tokio::{
    net::TcpListener,
    time::{sleep, Instant},
};
use tracing::{debug, info, instrument, level_filters::LevelFilter, warn};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

#[tokio::main]
async fn main() -> Result<()> {
    // 开始全局span
    // tracing_subscriber::fmt::init();

    // 创建每日滚动日志文件写入器
    let file_appender = tracing_appender::rolling::daily("./tmp/logs", "ecosystem.log");
    // 将其转换为非阻塞的写入器
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 创建一个控制台输出层Layer
    let console = fmt::Layer::new()
        .with_span_events(FmtSpan::CLOSE) // 在跟踪span关闭时输出日志，time.busy以及span所在的函数名等
        .pretty() // 是否以美观的方式打印
        .with_filter(LevelFilter::INFO); // 过滤info以上级别日志

    // 创建一个文件日志输出层
    let file = fmt::Layer::new()
        .with_writer(non_blocking) // 非阻塞输出到文件
        .pretty()
        .with_filter(LevelFilter::WARN); // 过滤warn以上级别日志

    // 使用控制台和文件日志输出层初始化日志订阅器，使得日志可以同时输出到控制台和文件
    tracing_subscriber::registry()
        .with(console) // 注册并初始化console layer
        .with(file)
        .init();

    let addr = "127.0.0.1:8080";

    let app = Router::new().route("/", get(index_handler));

    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", addr);

    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

// 开启span并跟踪trace
#[instrument]
async fn index_handler() -> &'static str {
    debug!("index handler started");
    sleep(Duration::from_millis(10)).await;
    let ret = long_task().await;
    // 输出一个info: index handler completed , http.status_text=200
    info!(http.status = 200, "index handler completed");
    ret
}

#[instrument]
async fn long_task() -> &'static str {
    let start = Instant::now();
    sleep(Duration::from_millis(112)).await;
    let elapsed = start.elapsed().as_millis() as u64;
    // 输出一个警告: task takes too long, app.task_duration=...
    warn!(app.task_duration = elapsed, "task takes too long");
    "Hello, World!"
}
