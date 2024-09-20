use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::Json;
use axum::{
    extract::State,
    routing::{get, patch},
    Router,
};
use chrono::{DateTime, Utc};
use serde::de::Visitor;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing::{info, instrument, level_filters::LevelFilter};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
};

#[derive(Debug, Clone, PartialEq)]
struct User {
    name: String,
    age: u8,
    dob: DateTime<Utc>, // 查看chrono的文档确定DataTime类型实现了serde trait
    skills: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserUpdate {
    age: Option<u8>,
    skills: Option<Vec<String>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // console追踪器
    let console = fmt::Layer::new()
        .with_span_events(FmtSpan::CLOSE)
        .pretty()
        .with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(console).init();

    let user = User {
        name: "zhangsan".to_string(),
        age: 18,
        // chrono 库的DateTime类型，是否可以serialize序列化，可以查看chrono库的文档
        // cargo add chrono --features=serde
        dob: Utc::now(),
        skills: vec!["java".to_string(), "python".to_string()],
    };
    let state = Arc::new(Mutex::new(user));
    let app = Router::new()
        .route("/", get(user_handler))
        .route("/", patch(update_handler))
        .with_state(state);

    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    info!("listening on {}", addr);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

#[instrument]
async fn user_handler(State(user): State<Arc<Mutex<User>>>) -> Json<User> {
    user.lock().unwrap().clone().into()
}

#[instrument]
async fn update_handler(
    State(user): State<Arc<Mutex<User>>>,
    Json(user_update): Json<UserUpdate>,
) -> Json<User> {
    let mut user = user.lock().unwrap();
    if let Some(age) = user_update.age {
        user.age = age;
    }
    if let Some(skills) = user_update.skills {
        user.skills = skills;
    }
    (*user).clone().into()
}

// impl Serialize/Deserialize   struct -> string
// 实际可以进入examples目录 cargo expand --example axum_serde查看实际serde库生成的代码
impl Serialize for User {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // serialize的结构，结构体名User，4个fields
        let mut state = serializer.serialize_struct("User", 4)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("age", &self.age)?;
        state.serialize_field("dob", &self.dob)?;
        state.serialize_field("skills", &self.skills)?;
        state.end()
    }
}

// string -> struct
impl<'de> Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<User, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // deserialize的结构体名，fields名以及要手动实现的 User 遍历器（次序遍历器，键值遍历器）
        deserializer.deserialize_struct("User", &["name", "age", "dob", "skills"], UserVisitor)
    }
}

struct UserVisitor;

impl<'de> Visitor<'de> for UserVisitor {
    type Value = User;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("struct User")
    }
    // 按次序遍历读取，这是当struct的fields声明为未命名时所用
    fn visit_seq<A>(self, mut seq: A) -> Result<User, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let name = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let age = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let dob = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
        let skills = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(3, &self))?;

        Ok(User {
            name,
            age,
            dob,
            skills,
        })
    }

    // 按key,value遍历读取，这是但struct的fields声明为命名时所用
    fn visit_map<A>(self, map: A) -> Result<User, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut name = None;
        let mut age = None;
        let mut dob = None;
        let mut skills = None;

        let mut map = map;
        while let Some(key) = map.next_key()? {
            match key {
                "name" => {
                    if name.is_some() {
                        return Err(serde::de::Error::duplicate_field("name"));
                    }
                    name = Some(map.next_value()?);
                }
                "age" => {
                    if age.is_some() {
                        return Err(serde::de::Error::duplicate_field("age"));
                    }
                    age = Some(map.next_value()?);
                }
                "dob" => {
                    if dob.is_some() {
                        return Err(serde::de::Error::duplicate_field("dob"));
                    }
                    dob = Some(map.next_value()?);
                }
                "skills" => {
                    if skills.is_some() {
                        return Err(serde::de::Error::duplicate_field("skills"));
                    }
                    skills = Some(map.next_value()?);
                }
                _ => {
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }

        let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
        let age = age.ok_or_else(|| serde::de::Error::missing_field("age"))?;
        let dob = dob.ok_or_else(|| serde::de::Error::missing_field("dob"))?;
        let skills = skills.ok_or_else(|| serde::de::Error::missing_field("skills"))?;

        Ok(User {
            name,
            age,
            dob,
            skills,
        })
    }
}
