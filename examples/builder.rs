use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};
use derive_builder::Builder;

#[allow(unused)]
// Builder生成器，为User生成关联的构建器UserBuilder
#[derive(Debug, Builder)]
// 指示derive_builder在为User生成构建器时，将构造函数的名称指定为_priv_build
#[builder(build_fn(name = "_priv_build"))]
struct User {
    // 在构建器中使用into方法设置该值
    #[builder(setter(into))]
    name: String,
    // 在构建器中使用into方法设置该值，可能为空，且有默认值None
    #[builder(setter(into, strip_option), default)]
    email: Option<String>,
    // 在构建器中使用自定义方法设置该值
    #[builder(setter(custom))]
    dob: DateTime<Utc>,
    //  在构建器中忽略该值
    #[builder(setter(skip))]
    age: u32,
    //  在构建器中默认值为vec![],使用的每一个使用skill方法设置该值
    #[builder(default = "vec![]", setter(each(name = "skill", into)))]
    skills: Vec<String>,
}

fn main() -> Result<()> {
    let user = User::build()
        .name("Alice")
        .skill("programming")
        .skill("debugging")
        .email("tyr@awesome.com")
        .dob("1990-01-01T00:00:00Z")
        .build()?;

    println!("{:?}", user);

    Ok(())
}

impl User {
    // 返回一个默认的构建器实例
    pub fn build() -> UserBuilder {
        UserBuilder::default()
    }
}

// Builder生成的关联的构建器类型UserBuilder
impl UserBuilder {
    // derive_builder为User构建了一个_priv_build方法，
    pub fn build(&self) -> Result<User> {
        let mut user = self._priv_build()?;
        user.age = (Utc::now().year() - user.dob.year()) as _;
        Ok(user)
    }
    // 为dob field自定义的构建方法
    pub fn dob(&mut self, value: &str) -> &mut Self {
        self.dob = DateTime::parse_from_rfc3339(value)
            .map(|dt| dt.with_timezone(&Utc))
            .ok();
        self
    }
}
