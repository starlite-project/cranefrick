mod local;

use std::{
	future::Future,
	iter::FusedIterator,
	ops::{Deref, DerefMut},
	pin::{Pin, pin},
	sync::{Arc, Mutex},
	task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt, stream::FusedStream};

pub use self::local::*;

#[repr(transparent)]
pub struct Iter<'a, T>(Data<'a, T>);

impl<'a, T> Iter<'a, T>
where
	T: Send + 'a,
{
	pub fn new<Fut>(f: impl FnOnce(IterContext<T>) -> Fut) -> Self
	where
		Fut: Future<Output = ()> + Send + Sync + 'a,
	{
		let value = Arc::new(Mutex::new(None));
		let cx = IterContext(Sender(value.clone()));
		let fut: Pin<Box<dyn Future<Output = ()> + Send + Sync + 'a>> = Box::pin(f(cx));
		let fut = Some(fut);
		Self(Data { value, fut })
	}
}

impl<T> FusedIterator for Iter<'_, T> {}

impl<T> Iterator for Iter<'_, T> {
	type Item = T;

	#[track_caller]
	fn next(&mut self) -> Option<Self::Item> {
		match self.0.poll_next(&mut Context::from_waker(Waker::noop())) {
			Poll::Ready(value) => value,
			Poll::Pending => panic!("ret was not called"),
		}
	}
}

#[repr(transparent)]
pub struct IterContext<T>(Sender<T>);

impl<T> IterContext<T>
where
	T: Send,
{
	#[track_caller]
	pub fn ret(&mut self, value: T) -> impl Future<Output = ()> + Send + Sync {
		self.0.set(value);
		&mut self.0
	}

	pub async fn ret_iter<I>(&mut self, iter: I)
	where
		I: IntoIterator<Item = T> + Send + Sync,
	{
		for value in iter {
			self.ret(value).await;
		}
	}
}

#[repr(transparent)]
pub struct AsyncIterContext<T>(IterContext<T>);

impl<T: Send> AsyncIterContext<T> {
	pub async fn ret_stream<S>(&mut self, stream: S)
	where
		S: Stream<Item = T> + Send,
	{
		let mut stream = pin!(stream);
		while let Some(value) = stream.next().await {
			self.0.ret(value).await;
		}
	}
}

impl<T> Deref for AsyncIterContext<T> {
	type Target = IterContext<T>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> DerefMut for AsyncIterContext<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[repr(transparent)]
pub struct AsyncIter<'a, T>(Iter<'a, T>);

impl<'a, T> AsyncIter<'a, T>
where
	T: 'a + Send,
{
	pub fn new<Fut>(f: impl FnOnce(AsyncIterContext<T>) -> Fut) -> Self
	where
		Fut: Future<Output = ()> + Send + Sync + 'a,
	{
		Self(Iter::new(|cx| f(AsyncIterContext(cx))))
	}
}

impl<T> FusedStream for AsyncIter<'_, T> {
	fn is_terminated(&self) -> bool {
		self.0.0.fut.is_none()
	}
}

impl<T> Stream for AsyncIter<'_, T> {
	type Item = T;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.0.0.poll_next(cx)
	}
}

struct Data<'a, T> {
	value: Arc<Mutex<Option<T>>>,
	fut: Option<Pin<Box<dyn Future<Output = ()> + Send + Sync + 'a>>>,
}

impl<T> Data<'_, T> {
	fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
		let Some(fut) = &mut self.fut else {
			return Poll::Ready(None);
		};

		let poll = fut.as_mut().poll(cx);
		match poll {
			Poll::Ready(()) => {
				assert!(self.value.lock().unwrap().is_none(), "ret was not awaited");
				self.fut = None;
				Poll::Ready(None)
			}
			Poll::Pending => {
				let value = self.value.lock().unwrap().take();
				if let Some(value) = value {
					Poll::Ready(Some(value))
				} else {
					Poll::Pending
				}
			}
		}
	}
}

#[repr(transparent)]
struct Sender<T>(Arc<Mutex<Option<T>>>);

impl<T> Sender<T> {
	#[track_caller]
	fn set(&self, value: T) {
		let mut guard = self.0.lock().unwrap();
		assert!(guard.is_none(), "ret was not awaited");
		*guard = Some(value);
	}
}

impl<T> Future for Sender<T> {
	type Output = ();

	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		if self.0.lock().unwrap().is_some() {
			Poll::Pending
		} else {
			Poll::Ready(())
		}
	}
}
