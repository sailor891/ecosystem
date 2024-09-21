use std::{thread, time::Duration};
use tokio::sync::mpsc;

// 同步运行时与异步运行时之间通过channel进行消息传递
// sync runtime  <== message transfer ==> async runtime
#[tokio::main]
async fn main() {
    // tokio task send string to expensive_blocking_task for execution
    let (tx, rx) = mpsc::channel(32);
    let handle = worker(rx);

    // 异步线程，发送任务到channel
    tokio::spawn(async move {
        let mut i = 0;
        loop {
            i += 1;
            println!("sending task {}", i);
            tx.send(format!("task {i}")).await.unwrap();
        }
    });

    handle.join().unwrap();
}

fn worker(mut rx: mpsc::Receiver<String>) -> thread::JoinHandle<()> {
    // 同步线程，接收任务，执行任务，返回结果
    thread::spawn(move || {
        let (sender, receiver) = std::sync::mpsc::channel();
        // 阻塞等待接收任务
        while let Some(s) = rx.blocking_recv() {
            let sender_clone = sender.clone();
            thread::spawn(move || {
                let ret = expensive_blocking_task(s);
                sender_clone.send(ret).unwrap();
            });
            let result = receiver.recv().unwrap();
            println!("result: {}", result);
        }
    })
}

fn expensive_blocking_task(s: String) -> String {
    thread::sleep(Duration::from_millis(800));
    blake3::hash(s.as_bytes()).to_string()
}
