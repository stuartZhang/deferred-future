#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ::deferred_future::ThreadDeferredFuture;
    use ::futures::{future, executor::{block_on, ThreadPool}, task::SpawnExt};
    use ::futures_time::{prelude::*, time::Duration};
    use ::std::{sync::PoisonError, time::Instant};
    block_on(async move {
        let deferred_future = ThreadDeferredFuture::default();
        let defer = deferred_future.defer();
        ThreadPool::new()?.spawn(async move {
            future::ready(()).delay(Duration::from_secs(1_u64)).await;
            let mut defer = defer.lock().unwrap_or_else(PoisonError::into_inner);
            defer.complete(Ok::<String, String>("1秒钟后才被延迟填入的消息".to_string()));
        })?;
        let start = Instant::now();
        let message = deferred_future.await;
        let end = Instant::now();
        let elapse = end.duration_since(start).as_secs();
        println!("为了收到消息<{}>，主协程先后等待了 {} 秒", message.unwrap(), elapse);
        Ok(())
    })
}
#[cfg(target_arch = "wasm32")]
fn main() {}