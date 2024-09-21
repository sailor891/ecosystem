use core::fmt;
use std::str::FromStr;

use anyhow::Result;
use axum::http;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chacha20poly1305::{
    aead::{Aead, OsRng},
    AeadCore, ChaCha20Poly1305, KeyInit,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

const KEY: &[u8] = b"01234567890123456789012345678901";

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
// 使用serde的属性宏，配置所有的fields在解析、解构时以camelCase格式化
#[serde(rename_all = "camelCase")]
struct User {
    name: String,
    // 解析、解构时重命名
    #[serde(rename = "privateAge")]
    age: u8,
    date_of_birth: DateTime<Utc>,
    // 如果为空，则不序列化
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    skills: Vec<String>,
    // 自定义类型
    state: WorkState,
    // data数据内容在解析、解构时以b64_encode、b64_decode函数进行编码、解码
    #[serde(serialize_with = "b64_encode", deserialize_with = "b64_decode")]
    data: Vec<u8>,
    // #[serde(
    //     serialize_with = "serialize_encrypt",
    //     deserialize_with = "deserialize_decrypt"
    // )]
    // SensitiveData手动实现了Display和FromStr trait；在解析、解构时使用
    #[serde_as(as = "DisplayFromStr")]
    sensitive: SensitiveData,
    // Uri在http这个crate中实现了Display和FromStr trait，指定Vec里面的Uri类型使用DisplayFromStr trait进行格式化
    // 声明嵌套类型的哪一个类型使用DisplayFromStr trait进行格式化
    #[serde_as(as = "Vec<DisplayFromStr>")]
    url: Vec<http::Uri>,
}

#[derive(Debug)]
struct SensitiveData(String);

#[derive(Debug, Serialize, Deserialize)]
// 使用 serde 的属性宏进一步配置枚举的序列化和反序列化行为。
// "rename_all = "camelCase"：指示在序列化和反序列化时，将字段名称转换为驼峰命名法。
// "tag = "type"：指定在序列化时，使用一个名为 "type" 的字段来区分不同的枚举变体。
// "content = "details"：表示枚举变体的具体内容将放在一个名为 "details" 的字段中。
#[serde(rename_all = "camelCase", tag = "type", content = "details")]
// 还有其它untagged_content等等
enum WorkState {
    Working(String),
    OnLeave(DateTime<Utc>),
    Terminated,
}

fn main() -> Result<()> {
    // let state = WorkState::Working("Rust Egineer".to_string());
    // 输出格式"state":{"type":"onLeave","details":"2024-09-21T05:42:02.267957800Z"},
    let state1 = WorkState::OnLeave(Utc::now());
    let user = User {
        name: "Alice".to_string(),
        age: 30,
        date_of_birth: Utc::now(),
        skills: vec!["Rust".to_string(), "Python".to_string()],
        state: state1,
        data: vec![1, 2, 3, 4, 5],
        sensitive: SensitiveData::new("secret"),
        url: vec!["https://example.com".parse()?],
    };

    let json = serde_json::to_string(&user)?;
    println!("{}", json);

    let user1: User = serde_json::from_str(&json)?;
    println!("{:?}", user1);
    println!("{:?}", user1.url[0].host());

    Ok(())
}

fn b64_encode<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let encoded = URL_SAFE_NO_PAD.encode(data);
    serializer.serialize_str(&encoded)
}

fn b64_decode<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // deserializer是Vec<u8>？？？先反序列化为字符串
    let encoded = String::deserialize(deserializer)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded.as_bytes())
        .map_err(serde::de::Error::custom)?;
    Ok(decoded)
}

#[allow(dead_code)]
fn serialize_encrypt<S>(data: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let encrypted = encrypt(data.as_bytes()).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&encrypted)
}

#[allow(dead_code)]
fn deserialize_decrypt<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let encrypted = String::deserialize(deserializer)?;
    let decrypted = decrypt(&encrypted).map_err(serde::de::Error::custom)?;
    let decrypted = String::from_utf8(decrypted).map_err(serde::de::Error::custom)?;
    Ok(decrypted)
}

/// encrypt with chacha20poly1305 and then encode with base64
fn encrypt(data: &[u8]) -> Result<String> {
    let cipher = ChaCha20Poly1305::new(KEY.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let ciphertext = cipher.encrypt(&nonce, data).unwrap();
    let nonce_cypertext: Vec<_> = nonce.iter().copied().chain(ciphertext).collect();
    let encoded = URL_SAFE_NO_PAD.encode(&nonce_cypertext);
    Ok(encoded)
}

/// decode with base64 and then decrypt with chacha20poly1305
fn decrypt(encoded: &str) -> Result<Vec<u8>> {
    let decoded = URL_SAFE_NO_PAD.decode(encoded.as_bytes())?;
    let cipher = ChaCha20Poly1305::new(KEY.into());
    let nonce = decoded[..12].into();
    let decrypted = cipher.decrypt(nonce, &decoded[12..]).unwrap();
    Ok(decrypted)
}

// SensitiveData 实现Display和FromStr trait，解析时使用display方法，解构时使用from_str方法
impl fmt::Display for SensitiveData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let encrypted = encrypt(self.0.as_bytes()).unwrap();
        write!(f, "{}", encrypted)
    }
}

impl FromStr for SensitiveData {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let decrypted = decrypt(s)?;
        let decrypted = String::from_utf8(decrypted)?;
        Ok(Self(decrypted))
    }
}

// SensitiveData的实例化str -> String -> FromStr.from_str -> SensitiveData
impl SensitiveData {
    fn new(data: impl Into<String>) -> Self {
        Self(data.into())
    }
}
