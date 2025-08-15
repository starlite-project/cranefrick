use copyless_test::BoxHelper;

fn main() {
	let z = foo_memcpy();
	println!("{:?}", &raw const *z);

	let z = foo_no_memcpy();
	println!("{:?}", &raw const *z);
}

pub enum Foo {
	Small(i8),
	Big([f32; 100]),
}

#[unsafe(no_mangle)]
#[inline(never)]
#[must_use]
pub fn foo_memcpy() -> Box<Foo> {
	Box::new(Foo::Big(std::array::from_fn(|x| x as f32)))
}

#[unsafe(no_mangle)]
#[inline(never)]
#[must_use]
pub fn foo_no_memcpy() -> Box<Foo> {
	Box::alloc().init(Foo::Big(std::array::from_fn(|x| x as f32)))
}
