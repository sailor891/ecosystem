use std::time::Duration;

use axum::{extract::Request, routing::get, Router};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{self, RandomIdGenerator, Tracer},
    Resource,
};
use tokio::{
    join,
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
async fn main() -> anyhow::Result<()> {
    // 开始全局span
    // tracing_subscriber::fmt::init();

    // 创建每日滚动日志文件写入器
    let file_appender = tracing_appender::rolling::daily("./tmp/logs", "ecosystem.log");
    // 将其转换为非阻塞的写入器
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 创建一个控制台输出层Layer,console layer for tracing_subscriber
    let console = fmt::Layer::new()
        .with_span_events(FmtSpan::CLOSE) // 在跟踪span关闭时输出日志，time.busy以及span所在的函数名等
        .pretty() // 是否以美观的方式打印
        .with_filter(LevelFilter::INFO); // 过滤info以上级别日志

    // 创建一个文件日志输出层,file layer for tracing_subscriber
    let file = fmt::Layer::new()
        .with_writer(non_blocking) // 非阻塞输出到文件
        .pretty()
        .with_filter(LevelFilter::WARN); // 过滤warn以上级别日志

    // 创建一个opentelemetry layer，opentelemetry tracing layer for tracing-subscriber
    let tracer = init_tracer()?;
    let opentelemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // 使用控制台和文件日志输出层初始化日志订阅器，使得日志可以同时输出到控制台和文件
    tracing_subscriber::registry()
        .with(console) // 注册并初始化console layer
        .with(file)
        .with(opentelemetry) // 注册opentelemetry tarcer
        .init();

    let addr = "127.0.0.1:8080";

    let app = Router::new().route("/", get(index_handler));

    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", addr);

    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

// 开启span并跟踪trace，每个span都会被记录
#[instrument(fields(http.uri = req.uri().path(), http.method = req.method().as_str()))]
async fn index_handler(req: Request) -> &'static str {
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
    // sleep(Duration::from_millis(120));
    // task1().await;
    // task2().await;
    // task3().await;
    // 使用jaegertracing 的jaeger ui查看执行过程，并发地优化代码

    // spawn multiple tasks
    let sl = sleep(Duration::from_millis(110));
    let t1 = task1();
    let t2 = task2();
    let t3 = task3();
    join!(sl, t1, t2, t3);

    let elapsed = start.elapsed().as_millis() as u64;
    // 输出一个警告: task takes too long, app.task_duration=...
    warn!(app.task_duration = elapsed, "task takes too long");
    "Hello, World!"
}

#[instrument]
async fn task1() {
    sleep(Duration::from_millis(10)).await;
}

#[instrument]
async fn task2() {
    sleep(Duration::from_millis(50)).await;
}

#[instrument]
async fn task3() {
    sleep(Duration::from_millis(30)).await;
}

// 初始化一个opentelemetry tracer
fn init_tracer() -> anyhow::Result<Tracer> {
    // 创建一个 OpenTelemetry 的追踪数据导出管道。该管道配置之后，将负责处理生成的追踪数据。
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing() // 表示pipeline用于trace 数据
        .with_exporter(
            // with_exporter配置导出器，这里表示使用otlp的的导出器
            opentelemetry_otlp::new_exporter()
                // 表示使用基于gRPC的tonic库来发送日志数据
                .tonic()
                // 配置导出日志数据的目标地址
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            // 配置tracing参数
            trace::config()
                // 设置ID生成器，用于生成追踪span的随机ID
                .with_id_generator(RandomIdGenerator::default())
                // 每个span最多记录32个事件
                .with_max_events_per_span(32)
                // 每个span最多附加64个属性
                .with_max_attributes_per_span(64)
                // 设置追踪资源的属性
                .with_resource(Resource::new(vec![KeyValue::new(
                    // Key: service.name表示追踪资源的名称 ,Value: axum-tracing
                    "service.name",
                    "axum-tracing",
                )])),
        )
        // 使用tokio运行时来安装追踪器
        .install_batch(runtime::Tokio)?;
    Ok(tracer)
}
