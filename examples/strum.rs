use anyhow::Result;
use serde::Serialize;
use serde_json::to_string;
use strum::{
    Display, EnumCount, EnumDiscriminants, EnumIs, EnumIter, EnumString, IntoEnumIterator,
    IntoStaticStr, VariantNames,
};

#[allow(unused)]
#[derive(Display, Debug, Serialize)]
enum Color {
    // 定义变体Red在serialize时输出为redred，to_string输出为red
    #[strum(serialize = "redred", to_string = "red")]
    Red,
    Green {
        range: usize,
    },
    Blue(usize),
    Yellow,
    // 添加to_string属性，自定义输出格式
    #[strum(to_string = "purple with {sat} saturation")]
    Purple {
        sat: usize,
    },
}

#[derive(
    Debug, EnumString, EnumCount, EnumDiscriminants, EnumIter, EnumIs, IntoStaticStr, VariantNames,
)]
#[allow(unused)]
enum MyEnum {
    A,
    B(String),
    C,
    D,
}

fn main() -> Result<()> {
    // VariantNames 提供支持，['A', 'B', 'C', 'D']
    println!("{:?}", MyEnum::VARIANTS);
    // EnumIter 提供支持
    MyEnum::iter().for_each(|v| println!("{:?}", v));
    // EnumCount 提供支持
    println!("total: {:?}", MyEnum::COUNT);

    let my_enum = MyEnum::B("hello".to_string());
    // EnumIs 提供支持
    println!("{:?}", my_enum.is_b());
    // EnumString 提供支持
    let s: &'static str = my_enum.into();
    println!("{}", s);

    let red = Color::Red;
    let green = Color::Green { range: 10 };
    let blue = Color::Blue(20);
    let yellow = Color::Yellow;
    let purple = Color::Purple { sat: 30 };

    println!(
        "red: {}, green: {}, blue: {}, yellow: {}, purple: {}",
        red, green, blue, yellow, purple
    );
    // serde_json结合strum::Display，自定义输出格式
    let red_str = to_string(&red)?;
    println!("{}", red_str);

    Ok(())
}
