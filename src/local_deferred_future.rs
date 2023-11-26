use ::futures::future::FusedFuture;
use ::std::{cell::{Cell, RefCell}, future::Future, pin::Pin, rc::Rc, task::{Context, Poll, Waker}};
#[derive(Default)]
pub struct LocalSharedState<T> {
    data: Option<T>,
    waker: Option<Waker>,
}
impl<T> LocalSharedState<T> {
    #[allow(unused)]
    pub fn complete(&mut self, data: T) {
        self.data.replace(data);
        self.waker.take().map(|waker| {waker.wake()});
    }
}
pub struct LocalDeferredFuture<T> {
    is_terminated: Cell<bool>,
    shared_state: Rc<RefCell<LocalSharedState<T>>>
}
impl<T> LocalDeferredFuture<T> {
    pub fn defer(&self) -> Rc<RefCell<LocalSharedState<T>>> {
        Rc::clone(&self.shared_state)
    }
}
impl<T> Default for LocalDeferredFuture<T> {
    fn default() -> Self {
        Self {
            is_terminated: Cell::new(false),
            shared_state: Rc::new(RefCell::new(LocalSharedState {
                data: None,
                waker: None
            }))
        }
    }
}
impl<T> Future for LocalDeferredFuture<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        let current_waker = context.waker();
        let mut shared_state = self.shared_state.borrow_mut();
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
impl<T> FusedFuture for LocalDeferredFuture<T> {
    fn is_terminated(&self) -> bool {
        self.is_terminated.get()
    }
}
