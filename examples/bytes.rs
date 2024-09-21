use anyhow::Result;
use bytes::{BufMut, BytesMut};

fn main() -> Result<()> {
    // 创建可变的字节缓冲区
    let mut buf = BytesMut::with_capacity(1024);
    // BytesMut类型可变，可以向buf实例添加数据
    buf.extend_from_slice(b"hello world\n");
    buf.put(&b"goodbye world"[..]);
    buf.put_i64(0xdeadbeef);

    println!("{:?}", buf);
    // split是浅复制,buf数据指针后移，len和cap做相应修改，不信可以看buf的cap
    let a = buf.split();
    println!("{:?},buf cap:{}", buf, buf.capacity());

    // 冻结缓冲区 BytesMut -> Bytes
    let mut b = a.freeze(); // inner data cannot be changed

    // b拥有缓冲区剩余部分，c拥有原实例的试图[0,at)
    let c = b.split_to(12);
    println!("{:?}", c);

    println!("{:?}", b);
    println!("{:?}", buf);

    Ok(())
}
