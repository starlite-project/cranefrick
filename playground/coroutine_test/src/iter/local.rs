#![allow(clippy::future_not_send)]

use std::{
	cell::RefCell,
	future::Future,
	iter::FusedIterator,
	ops::{Deref, DerefMut},
	pin::{Pin, pin},
	rc::Rc,
	task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt, stream::FusedStream};

#[repr(transparent)]
pub struct LocalIter<'a, T>(Data<'a, T>);

impl<'a, T: 'a> LocalIter<'a, T> {
	pub fn new<Fut>(f: impl FnOnce(LocalIterContext<T>) -> Fut) -> Self
	where
		Fut: Future<Output = ()> + 'a,
	{
		let value = Rc::new(RefCell::new(None));
		let cx = LocalIterContext(Sender(value.clone()));
		let fut: Pin<Box<dyn Future<Output = ()> + 'a>> = Box::pin(f(cx));
		let fut = Some(fut);
		Self(Data { value, fut })
	}
}

impl<T> FusedIterator for LocalIter<'_, T> {}

impl<T> Iterator for LocalIter<'_, T> {
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
pub struct LocalIterContext<T>(Sender<T>);

impl<T> LocalIterContext<T> {
	#[track_caller]
	pub fn ret(&mut self, value: T) -> impl Future<Output = ()> {
		self.0.set(value);
		&mut self.0
	}

	pub async fn ret_iter(&mut self, iter: impl IntoIterator<Item = T>) {
		for value in iter {
			self.ret(value).await;
		}
	}
}

#[repr(transparent)]
pub struct LocalAsyncIterContext<T>(LocalIterContext<T>);

impl<T> LocalAsyncIterContext<T> {
	pub async fn ret_stream(&mut self, stream: impl Stream<Item = T>) {
		let mut stream = pin!(stream);
		while let Some(value) = stream.next().await {
			self.0.ret(value).await;
		}
	}
}

impl<T> Deref for LocalAsyncIterContext<T> {
	type Target = LocalIterContext<T>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> DerefMut for LocalAsyncIterContext<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

#[repr(transparent)]
pub struct LocalAsyncIter<'a, T>(LocalIter<'a, T>);

impl<'a, T: 'a> LocalAsyncIter<'a, T> {
	pub fn new<Fut>(f: impl FnOnce(LocalAsyncIterContext<T>) -> Fut) -> Self
	where
		Fut: Future<Output = ()> + 'a,
	{
		Self(LocalIter::new(|cx| f(LocalAsyncIterContext(cx))))
	}
}

impl<T> FusedStream for LocalAsyncIter<'_, T> {
	fn is_terminated(&self) -> bool {
		self.0.0.fut.is_none()
	}
}

impl<T> Stream for LocalAsyncIter<'_, T> {
	type Item = T;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.0.0.poll_next(cx)
	}
}

#[repr(transparent)]
struct Sender<T>(Rc<RefCell<Option<T>>>);

impl<T> Sender<T> {
	#[track_caller]
	fn set(&self, value: T) {
		let mut data = self.0.borrow_mut();
		assert!(data.is_none(), "ret was not awaited");
		*data = Some(value);
	}
}

impl<T> Future for Sender<T> {
	type Output = ();

	fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
		if self.0.borrow().is_some() {
			Poll::Pending
		} else {
			Poll::Ready(())
		}
	}
}

struct Data<'a, T> {
	value: Rc<RefCell<Option<T>>>,
	fut: Option<Pin<Box<dyn Future<Output = ()> + 'a>>>,
}

impl<T> Data<'_, T> {
	fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
		let Some(fut) = &mut self.fut else {
			return Poll::Ready(None);
		};

		let poll = fut.as_mut().poll(cx);
		match poll {
			Poll::Ready(()) => {
				assert!(self.value.borrow().is_none(), "ret was not awaited");
				self.fut = None;
				Poll::Ready(None)
			}
			Poll::Pending => {
				if let Some(value) = self.value.borrow_mut().take() {
					Poll::Ready(Some(value))
				} else {
					Poll::Pending
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use std::{future::pending, ptr::null};

	use super::LocalIter;

	#[test]
	fn no_value() {
		let iter = LocalIter::<u32>::new(|mut _y| async move {});
		let list = iter.collect::<Vec<_>>();
		assert!(list.is_empty());
	}

	#[test]
	fn values() {
		let iter = LocalIter::new(|mut y| async move {
			eprintln!("yielding 1");
			y.ret(1).await;
			eprintln!("yielding 2");
			y.ret(2).await;
			eprintln!("done yielding");
		});

		let mut counter = 1;

		for i in iter {
			eprintln!("got {i}");
			assert_eq!(counter, i);

			counter += 1;
		}

		eprintln!("done counting");
	}

	#[test]
	fn values_ret_iter() {
		let iter = LocalIter::new(|mut y| async move {
			eprintln!("yielding 1 and 2");
			y.ret_iter([1, 2]).await;
			eprintln!("done yielding");
		});

		let mut counter = 1;

		for i in iter {
			eprintln!("got {i}");
			assert_eq!(counter, i);

			counter += 1;
		}

		eprintln!("done counting");
	}

	#[test]
	fn fused() {
		let mut iter = LocalIter::new(|mut y| async move {
			eprintln!("yielding 1");
			y.ret(1).await;
			eprintln!("yielding 2");
			y.ret(2).await;
			eprintln!("done yielding");
		});

		assert_eq!(iter.next(), Some(1));
		eprintln!("got 1");
		assert_eq!(iter.next(), Some(2));
		eprintln!("got 2");
		assert_eq!(iter.next(), None);
		assert_eq!(iter.next(), None);
	}

	#[test]
	fn values_with_lifetime() {
		let items = [1, 2];
		let items = &items;
		let iter = LocalIter::new(|mut y| async move {
			y.ret(&items[0]).await;
			y.ret(&items[1]).await;
		});

		let list = iter.collect::<Vec<_>>();

		assert_eq!(list, [&1, &2]);
	}

	#[test]
	#[should_panic = "ret was not called"]
	fn use_pending() {
		let iter = LocalIter::<u32>::new(|mut y| async move {
			y.ret(1).await;
			pending::<()>().await;
			y.ret(2).await;
		});

		_ = iter.collect::<Vec<_>>();
	}

	#[test]
	#[allow(unused_must_use)]
	#[should_panic = "ret was not awaited"]
	fn no_await_1() {
		let iter = LocalIter::new(|mut y| async move {
			y.ret(1);
		});

		_ = iter.collect::<Vec<_>>();
	}

	#[test]
	#[allow(unused_must_use)]
	#[should_panic = "ret was not awaited"]
	fn no_await_2() {
		let iter = LocalIter::new(|mut y| async move {
			y.ret(1);
			y.ret(2);
		});

		_ = iter.collect::<Vec<_>>();
	}

	#[test]
	fn check_not_send() {
		#[repr(transparent)]
		struct NotSend(#[allow(unused)] *const ());

		impl Drop for NotSend {
			fn drop(&mut self) {}
		}

		let iter = LocalIter::new(|mut y| async move {
			let _not_send = NotSend(null());
			y.ret(1).await;
			y.ret(2).await;
		});

		let list = iter.collect::<Vec<_>>();
		assert_eq!(list, [1, 2]);
	}
}

#[cfg(test)]
mod async_tests {
	use std::{ptr::null, time::Duration};

	use futures::{StreamExt, stream};
	use rt_local::runtime::core::test;

	use super::LocalAsyncIter;
	use crate::test_utils::sleep;

	#[test]
	async fn no_value() {
		let iter = LocalAsyncIter::<u32>::new(|mut _y| async move {});
		let list = iter.collect::<Vec<_>>().await;
		assert!(list.is_empty());
	}

	#[test]
	async fn values() {
		let iter = LocalAsyncIter::new(|mut y| async move {
			y.ret(1).await;
			y.ret(2).await;
		});

		let list = iter.collect::<Vec<_>>().await;

		assert_eq!(list, [1, 2]);
	}
}
