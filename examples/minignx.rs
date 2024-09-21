// it could be a proxy to a upstream
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::{
    io,
    net::{TcpListener, TcpStream},
};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

#[derive(Serialize, Deserialize, Clone)]
struct Config {
    upstream_addr: String,
    listen_addr: String,
}

// windows系统使用0.0.0.0:8080不行，该地址用于本地监听，不用于外部连接而127.0.0.1则是回环地址
// localhost会出现DNS解析问题，TcpStream::connect(upstream_addr)连接十分缓慢
// use tokio runtime to do a proxy server
#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let config = resolve_config();
    let config = Arc::new(config);
    info!("Upstream is {}", config.upstream_addr);
    info!("Listening on {}", config.listen_addr);

    let listener = TcpListener::bind(&config.listen_addr).await?;
    loop {
        let (client, addr) = listener.accept().await?;

        info!("Accepted connection from {}", addr);
        let cloned_config = config.clone();
        tokio::spawn(async move {
            let upstream = TcpStream::connect(&cloned_config.upstream_addr).await?;
            proxy(client, upstream).await?;
            Ok::<(), anyhow::Error>(())
        });
    }

    #[allow(unreachable_code)]
    Ok::<(), anyhow::Error>(())
}

async fn proxy(mut client: TcpStream, mut upstream: TcpStream) -> Result<()> {
    // 流分割client 和 upstream
    let (mut client_read, mut client_write) = client.split();
    let (mut upstream_read, mut upstream_write) = upstream.split();
    // 将数据从一个读取器复制到写入器
    // 创建了从客户端到上游服务端的数据复制任务
    let client_to_upstream = io::copy(&mut client_read, &mut upstream_write);
    // 创建了从上游服务端到客户端的数据复制任务
    let upstream_to_client = io::copy(&mut upstream_read, &mut client_write);
    match tokio::try_join!(client_to_upstream, upstream_to_client) {
        Ok((n, m)) => info!(
            "proxied {} bytes from client to upstream, {} bytes from upstream to client",
            n, m
        ),
        Err(e) => warn!("error proxying: {:?}", e),
    }
    Ok(())
}

fn resolve_config() -> Config {
    Config {
        upstream_addr: "127.0.0.1:8080".to_string(),
        listen_addr: "127.0.0.1:8081".to_string(),
    }
}
