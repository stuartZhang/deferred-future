# deferred-future

模仿[jQuery.Deferred()](https://api.jquery.com/jQuery.Deferred/)，允许

* 【地点】从`Future`实现类实例外部
* 【时间】异步地

改变当前`Future`对象的`Polling`状态从`Poll::Pending`至`Poll::Ready<T>`。这个痛点是[futures crate](https://docs.rs/futures/0.3.28/futures/future/index.html#functions)都**没有**照顾到的。

## 功能

`deferred-future crate`分别针对

* 单线程/`WASM`
* 多线程

提供了两套代码实现和两个自定义`cargo feature`：

|`cargo feature`|`FusedFuture`实现类|运行上下文|
|---------------|------------------|----------|
|`local` |`LocalDeferredFuture<T>` |单线程/`WASM`|
|`thread`|`ThreadDeferredFuture<T>`|多线程|

默认情况下，`local`与`thread`都处于开启状态。为了追求极致的编译时间（短）与输出二进制文件体积（小），屏蔽掉未被使用的模块非常有帮助。比如，在`WASM`工程内，启用【条件编译】和（编译时）“裁剪”依赖包是最明智的：

```toml
# 因为 WASM 不支持【操作系统线程】，所以仅只导入单线程代码实现
deferred-future = {version = "0.1.0", features = ["local"]}
```

另外，因为`deferred-future crate`选择实现[trait futures::future::FusedFuture](https://docs.rs/futures/0.3.29/futures/future/trait.FusedFuture.html)，而不仅只是来自【标准库】的[std::future::Future](https://doc.rust-lang.org/std/future/trait.Future.html)，所以其对更多“边界情况”提供了良好的容错支持。比如，

* 重复地`Polling`一个已经`Poll::Ready(T)`的`Future`实例不会导致`U.B.`。

## 安装

### 不开启【条件编译】

```shell
cargo add deferred-future
```

### 面向`WASM`，推荐仅开启`local`

```shell
cargo add deferred-future --features=local
```

## 用法

使用套路概括起来包括：

1. 构造一个`***DeferredFuture<T>`实例
   1. 在单线程上下文中，前缀`***`是`Local`
   2. 在多线程上下文中，前端`***`是`Thread`
   3. 泛型类型参数`T`对应于`Future::Output`关联类型 —— 代表了`Future`就绪后输出值的数据类型
      1. 在多线程上下文中，泛型类型参数`T`必须是`Send + Sync`的。
2. 从`***DeferredFuture<T>`实例抽取出`defer`属性值
   1. 被用来`Wake up`处于`Pending`状态`***DeferredFuture<T>`实例的`complete(T)`成员方法就隶属于此`defer`对象。
   2. 在单线程上下文中，`defer`是`Rc<RefCell<T>>`的引用计数·智能指针
   3. 在多线程上下文中，`defer`是`Arc<Mutex<T>>`的原子加锁引用计数·智能指针
3. 将`defer`对象克隆后甩到（另）一个异步任务`Task`块中去。
   1. 在异步块内，调用`defer`的`complete(T)`成员方法。
   2. 在单线程上下文中，`defer`对象需被**可修改**借入`defer.borrow_mut()`。
   3. 在多线程上下文中，需要先成功地获取线程同步锁`defer.lock().unwrap()`。
4. 在当前执行上下文，阻塞等待`***DeferredFuture<T>`实例就绪和返回结果。
   1. 就单线程而言，当前执行上下文即是“主线程”，和**同步**阻塞主线程。
   2. 就多线程而言，当前执行上下文就是“父异步块”，和**异步**阻塞上一级异步块。

下面仔细看代码例程。请特别留意注释说明。

### 单线程

```rust
use ::deferred_future::LocalDeferredFuture;
use ::futures::{future, executor::LocalPool, task::LocalSpawnExt};
use ::futures_time::{prelude::*, time::Duration};
use ::std::time::Instant;
// (1) 构造·形似 jQuery.Deferred() 的 trait FusedFuture 实现例类实例。
//     - 注意：泛型类型参数 —— `Future::Output`输出值类型是字符串。
let deferred_future = LocalDeferredFuture::default();
// (2) 取出它的 defer 实例。
let defer = deferred_future.defer();
// (3) 发起一个异步任务。在 2 秒钟后，填入`Future::Output`输出值。
let mut executor = LocalPool::new();
executor.spawner().spawn_local(async move {
    future::ready(()).delay(Duration::from_secs(2_u64)).await;
    // (3.1) 在异步块内，调用`defer`的`complete(T)`成员方法。
    defer.borrow_mut().complete("2秒钟后才被延迟填入的消息".to_string());
}).unwrap();
// (4) 同步阻塞主线程等待 #3 的异步任务执行结果，和抽取出`Future::Output`输出值。
let start = Instant::now();
let message = executor.run_until(deferred_future); // (4.1) 会造成主线程的同步阻塞
let end = Instant::now();
let elapse = end.duration_since(start).as_secs();
println!("为了收到消息<{}>，主协程先后等待了 {} 秒", message, elapse);
```

从命令行，执行命令`cargo.exe run --example local-usage`可直接运行此例程。

### 多线程

```rust
use ::deferred_future::ThreadDeferredFuture;
use ::futures::{future, executor::{block_on, ThreadPool}, task::SpawnExt};
use ::futures_time::{prelude::*, time::Duration};
use ::std::{error::Error, sync::PoisonError, time::Instant};
block_on(async move {
    // (1) 构造·形似 jQuery.Deferred() 的 trait FusedFuture 实现类实例。
    //     - 注意：泛型类型参数 —— `Future::Output`输出值类型是字符串。
    //     - String 是 Send + Sync 的数据类型，和支持跨线程传递的。
    let deferred_future = ThreadDeferredFuture::default();
    // (2) 取出它的 defer 实例。
    let defer = deferred_future.defer();
    // (3) 发起一个异步任务。在 1 秒钟后，填入`Future::Output`输出值。
    ThreadPool::new()?.spawn(async move {
        future::ready(()).delay(Duration::from_secs(1_u64)).await;
        // (3.1) 在异步块内，调用`defer`的`complete(T)`成员方法。
        let mut defer = defer.lock().unwrap_or_else(PoisonError::into_inner);
        defer.complete("1秒钟后才被延迟填入的消息".to_string());
    })?;
    // (4) 异步阻塞当前 Task 等待 #3 的异步任务执行结果，和抽取出`Future::Output`输出值。
    let start = Instant::now();
    let message = deferred_future.await; // (4.1) 会造成上一级异步块的异步阻塞
    let end = Instant::now();
    let elapse = end.duration_since(start).as_secs();
    println!("为了收到消息<{}>，主协程先后等待了 {} 秒", message, elapse);
    Ok(())
})?;
```

从命令行，执行命令`cargo.exe run --example thread-usage`可直接运行此例程。

### `WASM`

```rust
use ::deferred_future::LocalDeferredFuture;
use ::futures::future;
use ::wasm_gloo_dom_events::EventStream;
// (1) 构造·形似 jQuery.Deferred() 的 trait FusedFuture 实例类实例。
//     - 注意：泛型类型参数 —— `Future::Output`输出值类型是 u32。
let deferred_future = LocalDeferredFuture::default();
// (2) 取出它的 defer 实例。
let defer = deferred_future.defer();
// (3) 创建一个计划任务。仅当计划任务被执行时，才填入`Future::Output`输出值。
let _ = EventStream::on_timeout("test", 1000, move |_event| {
    // (3.1) 在计划任务处理函数内，调用`defer`的`complete(T)`成员方法。
    defer.borrow_mut().complete("12".to_string());
    future::ready(Ok(()))
});
// (4) 异步阻塞当前 Task 等待 #3 的按钮点击事件的发生，和抽取出`Future::Output`输出值。
let result = deferred_future.await;
assert_eq!(result, "12");
```

从命令行，执行命令`wasm-pack test --chrome`可直接运行此例程。

#### "无头"运行模式

```shell
# 开启详细日志
set RUST_LOG=wasm_bindgen_test_runner
# 运行 webdriver
wasm-pack test --chrome --headless --chromedriver=<chromedriver可执行文件完整路径>
```

注意：需要本地安装的

* `webdriver`与
* `Chrome`

版本匹配。否则，就会收到`"error":"invalid session id"`的反馈。查看详细日志就会发现这么一条`DEBUG`日志`"message":"session not created: This version of ChromeDriver only supports Chrome version 114\nCurrent browser version is 118.0.5993.118 with binary path`。可从这[下载](https://googlechromelabs.github.io/chrome-for-testing/)最新版的`ChromeDriver`。
