use std::{
	future::Future,
	pin::{Pin, pin},
	sync::Arc,
	task::{Context, Poll, Wake, Waker},
	thread::{self, Thread},
};

use color_eyre::Result;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

fn main() -> Result<()> {
	install_tracing();
	color_eyre::install()?;

	let fut = async {
		tracing::info!("Inside a future");

		yield_now().await;
	};

	tracing::info!("Waiting for future");

	block_on(fut);

	tracing::info!("Done with future");

	Ok(())
}

#[repr(transparent)]
struct ThreadWaker(Thread);

impl Wake for ThreadWaker {
	fn wake(self: Arc<Self>) {
		tracing::trace!("notified");
		self.0.unpark();
	}
}

fn block_on<T>(fut: impl Future<Output = T>) -> T {
	let span = tracing::trace_span!("block_on");
	let _enter = span.enter();

	let mut fut = pin!(fut);

	let t = thread::current();
	let waker = Arc::new(ThreadWaker(t)).into();
	let mut cx = Context::from_waker(&waker);

	loop {
		match fut.as_mut().poll(&mut cx) {
			Poll::Ready(res) => {
				tracing::trace!("completed");

				break res;
			}
			Poll::Pending => {
				tracing::trace!("pending until notification");

				thread::park();
			}
		}
	}
}

async fn yield_now() {
	struct YieldNow(bool);

	impl Future for YieldNow {
		type Output = ();

		fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
			if self.0 {
				Poll::Ready(())
			} else {
				self.0 = true;
				let _waker = WakeOnDrop(cx.waker());
				Poll::Pending
			}
		}
	}

	YieldNow(false).await;
}

struct WakeOnDrop<'a>(&'a Waker);

impl Drop for WakeOnDrop<'_> {
	fn drop(&mut self) {
		self.0.wake_by_ref();
	}
}

fn install_tracing() {
	let fmt_layer = tracing_subscriber::fmt::layer()
		.with_target(true)
		.with_thread_ids(true)
		.with_thread_names(true);

	tracing_subscriber::registry().with(fmt_layer).init();
}
