#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]

use std::{
	future::Future,
	pin::pin,
	sync::Arc,
	task::{Context, Poll, Wake},
	thread::{self, Thread},
};

pub fn block_on<T>(fut: impl Future<Output = T>) -> T {

	let mut fut = pin!(fut);

	let t = thread::current();
	let waker = Arc::new(ThreadWaker(t)).into();
	let mut cx = Context::from_waker(&waker);

	loop {
		tracing::info!(target: "test::waker", op = "poll");
		match fut.as_mut().poll(&mut cx) {
			Poll::Ready(res) => return res,
			Poll::Pending => thread::park(),
		}
	}
}

#[repr(transparent)]
struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
	fn wake(self: Arc<Self>) {
		self.0.unpark();
	}
}
