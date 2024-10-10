// #[cfg(test)]
// mod tests {
//     use loom::sync::atomic::AtomicUsize;
//     use loom::sync::atomic::Ordering::{Acquire, Relaxed, Release};
//     use loom::sync::Arc;
//     use loom::thread;

//     #[test]
//     #[should_panic]
//     fn buggy_concurrent_inc() {
//         // 使用loom来进行concurrency并发代码的测试
//         loom::model(|| {
//             let num = Arc::new(AtomicUsize::new(0));

//             let nth: Vec<_> = (0..2)
//                 .map(|_| {
//                     let num = num.clone();
//                     thread::spawn(move || {
//                         let curr = num.load(Acquire);
//                         // This is a bug! this is not atomic!
//                         num.store(curr + 1, Release);

//                         // fix
//                         // num.fetch_add(1, Relaxed);
//                     })
//                 })
//                 .collect();

//             for th in nth {
//                 th.join().unwrap();
//             }
//             println!("num:{:?}", num);
//             assert_eq!(2, num.load(Relaxed));
//         });
//     }
// }
