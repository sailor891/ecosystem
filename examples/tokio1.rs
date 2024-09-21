use std::{thread, time::Duration};

use tokio::{
    fs,
    runtime::{Builder, Runtime},
    time::sleep,
};

// 在同步环境中，创建异步运行时tokio并运行异步任务
// single-threaded下的tokio runtime，阐释tokio的runtime，以及可以cargo expand查看tokio::main的展开
fn main() {
    let handle = thread::spawn(|| {
        // 构建tokio运行时，在当前线程中运行异步任务，enable_all开启全部tokio功能
        let rt = Builder::new_current_thread().enable_all().build().unwrap();

        // rt.block_on(run(&rt));

        // 在tokio运行时中启动一个异步任务，相当于把异步任务放入Tokio的run queue
        rt.spawn(async {
            sleep(Duration::from_millis(500)).await;
            println!("spawn on thread");
        });

        rt.spawn(async {
            println!("future 1");
            let content = fs::read("Cargo.toml").await.unwrap();
            println!("content: {:?}", content.len());
        });

        rt.spawn(async {
            println!("future 2");
            let result = expensive_blocking_task("hello".to_string());
            println!("result: {}", result);
        });

        // block_on注释线程，等待下面这个异步任务执行完毕
        rt.block_on(async {
            sleep(Duration::from_secs(1)).await;
            println!("future 3");
        })
    });

    handle.join().unwrap();
}

fn expensive_blocking_task(s: String) -> String {
    thread::sleep(Duration::from_millis(800));
    blake3::hash(s.as_bytes()).to_string()
}

#[allow(unused)]
async fn run(rt: &Runtime) {
    rt.spawn(async {
        println!("future 1");
        let content = fs::read("Cargo.toml").await.unwrap();
        println!("content: {:?}", content.len());
    });
    rt.spawn(async {
        println!("future 2");
        let result = expensive_blocking_task("hello".to_string());
        println!("result: {}", result);
    });
    sleep(Duration::from_secs(1)).await;
}
