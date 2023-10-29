
#[cfg(target_arch = "wasm32")]
use ::wasm_bindgen_test::*;
#[cfg(all(not(feature = "nodejs"), target_arch = "wasm32"))]
wasm_bindgen_test_configure!(run_in_browser);
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
#[cfg(target_arch = "wasm32")]
async fn deferred_future_test() {
    use ::deferred_future::LocalDeferredFuture;
    use ::futures::future;
    use ::wasm_gloo_dom_events::EventStream;
    let deferred_future = LocalDeferredFuture::default();
    let defer = deferred_future.defer();
    let _ = EventStream::on_timeout("test", 1000, move |_event| {
        defer.borrow_mut().complete("12".to_string());
        future::ready(Ok(()))
    });
    let result = deferred_future.await;
    assert_eq!(result, "12");
}
