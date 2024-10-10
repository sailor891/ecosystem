use anyhow::Result;
use dashmap::DashMap;
use futures::{stream::SplitStream, SinkExt, StreamExt};
use std::{fmt, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{info, warn};
// use tracing::{info, level_filters::LevelFilter, warn};
// use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

const MAX_MESSAGES: usize = 128;

#[derive(Debug, Default)]
struct State {
    peers: DashMap<SocketAddr, mpsc::Sender<Arc<Message>>>,
}

#[derive(Debug)]
struct Peer {
    username: String,
    stream: SplitStream<Framed<TcpStream, LinesCodec>>,
}

#[derive(Debug)]
enum Message {
    UserJoined(String),
    UserLeft(String),
    Chat { sender: String, content: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    // let layer = Layer::new().with_filter(LevelFilter::INFO);
    // tracing_subscriber::registry().with(layer).init();

    // 要安装tokio-console cargo install tokio-console 和开启服务tokio-console
    // 添加console_subscriber依赖
    // 启动console要配置临时环境变量 $env:RUSTFLAGS="--cfg tokio_unstable"再运行
    console_subscriber::init();
    // 启用tokio-console服务即可查看tokio的任务信息

    // 建立监听端口
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    info!("Starting chat server on {}", addr);
    let state = Arc::new(State::default());

    // 循环接收处理listener监听器，并传入handle_client处理
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Accepted connection from: {}", addr);
        let state_cloned = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(state_cloned, addr, stream).await {
                warn!("Failed to handle client {}: {}", addr, e);
            }
        });
    }
}

async fn handle_client(state: Arc<State>, addr: SocketAddr, stream: TcpStream) -> Result<()> {
    // 创建一个LinesCodec编解码器，将TCP流包装为LinesCodec编解码的Frame，并返回一个Framed对象
    let mut stream = Framed::new(stream, LinesCodec::new());
    // 使用TCP流向客户端发送欢迎信息
    stream.send("Enter your username:").await?;

    let username = match stream.next().await {
        Some(Ok(username)) => username,
        Some(Err(e)) => return Err(e.into()),
        None => return Ok(()),
    };
    // username和stream封装到Peer结构体中，将stream分割为发送和接收流
    // 将username和向客户端发送消息的stream封装到Peer结构体中
    let mut peer = state.add(addr, username, stream).await;

    // addr和message将消息广播给其它节点
    let message = Arc::new(Message::user_joined(&peer.username));
    info!("{}", message);
    state.broadcast(addr, message).await;

    // 持续处理client 2 serve的消息,peer.stream==tcp_stream_receiver接收client发送过来的消息
    while let Some(line) = peer.stream.next().await {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                warn!("Failed to read line from {}: {}", addr, e);
                break;
            }
        };

        // 组装消息，将消息广播给其它user
        let message = Arc::new(Message::chat(&peer.username, line));
        state.broadcast(addr, message).await;
    }

    // 当运行到这行代码时，说明这个peer退出chat系统，要在全局state中移除这个peer
    state.peers.remove(&addr);

    // 向其他peer发送这个user离开chat系统的消息
    let message = Arc::new(Message::user_left(&peer.username));
    info!("{}", message);
    state.broadcast(addr, message).await;

    Ok(())
}

impl State {
    async fn broadcast(&self, addr: SocketAddr, message: Arc<Message>) {
        for peer in self.peers.iter() {
            if peer.key() == &addr {
                continue;
            }
            // 向其它user的channel的sender发送消息
            if let Err(e) = peer.value().send(message.clone()).await {
                warn!("Failed to send message to {}: {}", peer.key(), e);
                // if send failed, peer might be gone, remove peer from state
                self.peers.remove(peer.key());
            }
        }
    }

    async fn add(
        &self,
        addr: SocketAddr,
        username: String,
        stream: Framed<TcpStream, LinesCodec>,
    ) -> Peer {
        // 给每一个用户创建一个发送通道，serve将发送的消息给到发送通道，发送通道接收到消息后发送
        let (tx, mut rx) = mpsc::channel(MAX_MESSAGES);
        self.peers.insert(addr, tx);

        // 分割stream为发送和接收流，使用发送流向用户发送消息
        let (mut stream_sender, stream_receiver) = stream.split();

        // 接收消息的通道，当channel接收到消息时，将消息使用stream_sender发送给客户端
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = stream_sender.send(message.to_string()).await {
                    warn!("Failed to send message to {}: {}", addr, e);
                    break;
                }
            }
        });

        // return peer
        Peer {
            username,
            stream: stream_receiver,
        }
    }
}

impl Message {
    fn user_joined(username: &str) -> Self {
        let content = format!("{} has joined the chat", username);
        Self::UserJoined(content)
    }

    fn user_left(username: &str) -> Self {
        let content = format!("{} has left the chat", username);
        Self::UserLeft(content)
    }

    fn chat(sender: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Chat {
            sender: sender.into(),
            content: content.into(),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserJoined(content) => write!(f, "[{}]", content),
            Self::UserLeft(content) => write!(f, "[{} :(]", content),
            Self::Chat { sender, content } => write!(f, "{}: {}", sender, content),
        }
    }
}
