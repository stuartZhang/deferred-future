use ::futures::future::FusedFuture;
use ::std::{cell::Cell, future::Future, pin::Pin, sync::{Arc, Mutex, PoisonError}, task::{Context, Poll, Waker}};
#[derive(Default)]
pub struct ThreadSharedState<T>
where T: Send + Sync {
    data: Option<T>,
    waker: Option<Waker>,
}
impl<T> ThreadSharedState<T>
where T: Send + Sync {
    #[allow(unused)]
    pub fn complete(&mut self, data: T) {
        self.data.replace(data);
        self.waker.take().map(|waker| {waker.wake()});
    }
}
pub struct ThreadDeferredFuture<T>
where T: Send + Sync {
    is_terminated: Cell<bool>,
    shared_state: Arc<Mutex<ThreadSharedState<T>>>
}
impl<T> ThreadDeferredFuture<T>
where T: Send + Sync {
    pub fn defer(&self) -> Arc<Mutex<ThreadSharedState<T>>> {
        Arc::clone(&self.shared_state)
    }
}
impl<T> Default for ThreadDeferredFuture<T>
where T: Send + Sync {
    fn default() -> Self {
        Self {
            is_terminated: Cell::new(false),
            shared_state: Arc::new(Mutex::new(ThreadSharedState {
                data: None,
                waker: None
            }))
        }
    }
}
impl<T> Future for ThreadDeferredFuture<T>
where T: Send + Sync {
    type Output = T;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let current_waker = context.waker();
        let mut shared_state = self.shared_state.lock().unwrap_or_else(PoisonError::into_inner);
        if shared_state.waker.as_ref().map_or(true, |w| !w.will_wake(current_waker)) {
            shared_state.waker.replace(current_waker.clone());
        }
        if shared_state.data.is_none() {
            self.is_terminated.set(false);
            Poll::Pending
        } else {
            self.is_terminated.set(true);
            Poll::Ready(shared_state.data.take().unwrap())
        }
    }
}
impl<T> FusedFuture for ThreadDeferredFuture<T>
where T: Send + Sync {
    fn is_terminated(&self) -> bool {
        self.is_terminated.get()
    }
}
