use std::{
	sync::{
		Condvar, Mutex,
		atomic::{AtomicBool, Ordering},
	},
	time::Duration,
};

pub struct Notification {
	has_been_notified: AtomicBool,
	mutex: Mutex<()>,
	cv: Condvar,
}

impl Notification {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	pub fn wait(&self) {
		if self.has_been_notified() {
			return;
		}

		let mut lock = self.mutex.lock().unwrap();
		while !self.has_been_notified() {
			lock = self.cv.wait(lock).unwrap();
		}
	}

	pub fn wait_with_timeout(&self, timeout: Duration) -> bool {
		if self.has_been_notified() {
			return true;
		}

		let mut lock = self.mutex.lock().unwrap();
		while !self.has_been_notified() {
			let (next, result) = self.cv.wait_timeout(lock, timeout).unwrap();
			if result.timed_out() {
				return false;
			}

			lock = next;
		}

		self.has_been_notified()
	}

	pub fn notify(&self) {
		let _guard = self.mutex.lock().unwrap();
		self.has_been_notified.store(true, Ordering::Release);
		self.cv.notify_all();
	}

	pub fn has_been_notified(&self) -> bool {
		self.has_been_notified.load(Ordering::Acquire)
	}
}

impl Default for Notification {
	fn default() -> Self {
		Self {
			has_been_notified: AtomicBool::new(false),
			mutex: Mutex::new(()),
			cv: Condvar::new(),
		}
	}
}

impl Drop for Notification {
	fn drop(&mut self) {
		let _guard = self.mutex.lock();
	}
}

#[cfg(test)]
mod tests {
	use std::{
		sync::{
			Arc,
			atomic::{AtomicUsize, Ordering},
		},
		thread,
		time::Duration,
	};

	use super::Notification;

	#[test]
	fn same_thread() {
		let n = Notification::default();
		n.notify();
		n.wait();
	}

	#[test]
	fn wakes_up_multiple() {
		let n = Arc::new(Notification::default());
		let ctr = Arc::new(AtomicUsize::new(0));

		let threads = (0..20)
			.map(|_| {
				let n = n.clone();
				let ctr = ctr.clone();
				thread::spawn(move || {
					n.wait();
					ctr.fetch_add(1, Ordering::SeqCst);
				})
			})
			.collect::<Vec<_>>();

		thread::sleep(Duration::from_millis(100));
		assert_eq!(ctr.load(Ordering::SeqCst), 0);

		n.notify();
		for t in threads {
			t.join().unwrap();
		}

		assert_eq!(ctr.load(Ordering::SeqCst), 20);
	}

	#[test]
	fn times_out() {
		let n = Arc::new(Notification::default());
		let threads = (0..20)
			.map(|_| {
				let n = n.clone();
				thread::spawn(move || assert!(!n.wait_with_timeout(Duration::from_millis(10))))
			})
			.collect::<Vec<_>>();

		for t in threads {
			t.join().unwrap();
		}
	}

	#[test]
	fn race() {
		let n = Arc::new(Notification::default());
		let ctr = Arc::new(AtomicUsize::new(0));
		let threads = (0..20)
			.map(|i| {
				let n = n.clone();
				let ctr = ctr.clone();
				thread::spawn(move || {
					if matches!(i, 19) {
						n.notify();
					} else {
						n.wait();
					}

					ctr.fetch_add(1, Ordering::SeqCst);
				})
			})
			.collect::<Vec<_>>();

		for t in threads {
			t.join().unwrap();
		}

		assert_eq!(ctr.load(Ordering::SeqCst), 20);
	}
}
