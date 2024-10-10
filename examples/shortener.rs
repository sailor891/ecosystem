use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use http::{header::LOCATION, HeaderMap, StatusCode};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

// 派生Deserialize，解构Req
#[derive(Debug, Deserialize)]
struct ShortenReq {
    url: String,
}

// 派生Serialize，解析Res
#[derive(Debug, Serialize)]
struct ShortenRes {
    url: String,
}

#[derive(Debug, Clone)]
struct AppState {
    db: PgPool,
}

// 派生FromRow，数据模型model与数据库进行双向解析、解构
#[derive(Debug, FromRow)]
struct UrlRecord {
    #[sqlx(default)]
    id: String,
    #[sqlx(default)]
    url: String,
}

const LISTEN_ADDR: &str = "127.0.0.1:9876";

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    // 在全局状态中保存postgres的连接池
    let url = "postgres://postgres:123456@localhost:5432/shortener";
    let state = AppState::try_new(url).await?;
    info!("Connected to database: {url}");

    // 监听服务地址
    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    info!("Listening on: {}", LISTEN_ADDR);

    // 注册uri路由器
    let app = Router::new()
        .route("/", post(shorten))
        .route("/:id", get(redirect))
        .with_state(state);

    // 启动axum框架的服务器，传入监听地址以及路由实例以及处理函数
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

// 基于axum的handler，它的参数有顺序要求。http的header里的uri、参数等可以在全局State前，而body只能在State后面
async fn shorten(
    State(state): State<AppState>,
    Json(data): Json<ShortenReq>,
) -> Result<impl IntoResponse, StatusCode> {
    // 使用json的deserialize解构请求的data内容，解构为ShortenReq
    // data为解构json类型的body的映射类型
    let id = state.shorten(&data.url).await.map_err(|e| {
        warn!("Failed to shorten URL: {e}");
        StatusCode::UNPROCESSABLE_ENTITY
    })?;
    // 返回json格式的body，其中包括一个url数据
    let body = Json(ShortenRes {
        url: format!("http://{}/{}", LISTEN_ADDR, id),
    });
    Ok((StatusCode::CREATED, body))
}

// url在http请求头里面，可以重用的数据，可以放在全局State前
async fn redirect(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    // get_url向postgres数据库查询 path 中指定的id的url
    let url = state
        .get_url(&id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // 声明一个字段为空的http的header
    let mut headers = HeaderMap::new();

    // 给header插入location字段，值为可解析的url
    headers.insert(LOCATION, url.parse().unwrap());

    // 返回的http状态码：Redirect（重定向），重定向的地址为http文本中的location字段值
    // 客户端client接收到这个response之后，会重定向到location的url
    Ok((StatusCode::PERMANENT_REDIRECT, headers))
}

impl AppState {
    async fn try_new(url: &str) -> Result<Self> {
        // 使用sqlx的postgres驱动连接postgres数据库
        let pool = PgPool::connect(url).await?;
        // sqlx的query注册sql语句，execute(&pool)指定执行的数据库
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS urls (
                id CHAR(6) PRIMARY KEY,
                url TEXT NOT NULL UNIQUE
            )
            "#,
        )
        .execute(&pool)
        .await?;
        Ok(Self { db: pool })
    }

    async fn shorten(&self, url: &str) -> Result<String> {
        // 给url生成随机id
        let id = nanoid!(6);

        // 在查询返回fetch_one的情况下，可以指定ret的映射类型 UrlRecord\
        // 占位符$1 和 $2，当url冲突时，do执行update set url=EXCLUDED.url RETURNING id
        let ret: UrlRecord = sqlx::query_as(
            "INSERT INTO urls (id, url) VALUES ($1, $2) ON CONFLICT(url) DO UPDATE SET url=EXCLUDED.url RETURNING id",
        )
        .bind(&id)
        .bind(url)
        .fetch_one(&self.db)
        .await?;
        Ok(ret.id)
    }

    async fn get_url(&self, id: &str) -> anyhow::Result<String> {
        // fetch_one返回，指定映射数据类型 UrlRecord
        let ret: UrlRecord = sqlx::query_as("SELECT url FROM urls WHERE id = $1")
            .bind(id)
            .fetch_one(&self.db)
            .await?;
        Ok(ret.url)
    }
}
