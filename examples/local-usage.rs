use ::deferred_future::LocalDeferredFuture;
use ::futures::{future, executor::LocalPool, task::LocalSpawnExt};
use ::futures_time::{prelude::*, time::Duration};
use ::std::time::Instant;
fn main() {
    let deferred_future: LocalDeferredFuture<String> = LocalDeferredFuture::default();
    let defer = deferred_future.defer();
    let mut executor = LocalPool::new();
    executor.spawner().spawn_local(async move {
        future::ready(()).delay(Duration::from_secs(2_u64)).await;
        defer.borrow_mut().complete("2秒钟后才被延迟填入的消息".to_string());
    }).unwrap();
    let start = Instant::now();
    let message = executor.run_until(deferred_future);
    let end = Instant::now();
    let elapse = end.duration_since(start).as_secs();
    println!("为了收到消息<{}>，主协程先后等待了 {} 秒", message, elapse);
}