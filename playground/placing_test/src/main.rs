use placing::placing;

fn main() {
	let mut cat = unsafe { Cat::placing_uninit_new() };
	unsafe {
		cat.placing_init_new(12);
	}
	assert_eq!(cat.age(), 12);
}

#[placing]
struct Cat {
	age: u8,
}

#[placing]
impl Cat {
	#[placing]
	fn new(age: u8) -> Self {
		Self { age }
	}

	fn age(&self) -> u8 {
		self.age
	}
}
