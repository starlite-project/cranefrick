use std::{
	cell::RefCell,
	ptr::{self, NonNull},
	rc::Rc,
};

use inkwell::memory_manager::McjitMemoryManager;

const CAPACITY_IN_BYTES: usize = 1024 * 128;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct MemoryManager {
	data: Rc<RefCell<MemoryManagerData>>,
}

impl MemoryManager {
	pub fn new() -> Self {
		let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize };
		let code_buffer_pointer = unsafe {
			NonNull::new_unchecked(
				libc::mmap(
					ptr::null_mut(),
					CAPACITY_IN_BYTES,
					libc::PROT_READ | libc::PROT_WRITE,
					libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
					-1,
					0,
				)
				.cast::<u8>(),
			)
		};

		let data_buffer_pointer = unsafe {
			NonNull::new_unchecked(
				libc::mmap(
					ptr::null_mut(),
					CAPACITY_IN_BYTES,
					libc::PROT_READ | libc::PROT_WRITE,
					libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
					-1,
					0,
				)
				.cast::<u8>(),
			)
		};

		Self {
			data: Rc::new(RefCell::new(MemoryManagerData {
				fixed_capacity_in_bytes: CAPACITY_IN_BYTES,
				fixed_page_size: page_size,
				code_buffer_pointer,
				code_offset: 0,
				data_buffer_pointer,
				data_offset: 0,
			})),
		}
	}
}

impl McjitMemoryManager for MemoryManager {
	fn allocate_code_section(
		&mut self,
		size: libc::uintptr_t,
		_: libc::c_uint,
		_: libc::c_uint,
		_: &str,
	) -> *mut u8 {
		let mut data = self.data.borrow_mut();

		let alloc_size = size.div_ceil(data.fixed_page_size) * data.fixed_page_size;
		let ptr = unsafe { data.code_buffer_pointer.as_ptr().add(data.code_offset) };
		data.code_offset += alloc_size;

		ptr
	}

	fn allocate_data_section(
		&mut self,
		size: libc::uintptr_t,
		_: libc::c_uint,
		_: libc::c_uint,
		_: &str,
		_: bool,
	) -> *mut u8 {
		let mut data = self.data.borrow_mut();

		let alloc_size = size.div_ceil(data.fixed_page_size) * data.fixed_page_size;
		let ptr = unsafe { data.data_buffer_pointer.as_ptr().add(data.data_offset) };
		data.data_offset += alloc_size;

		ptr
	}

	fn finalize_memory(&mut self) -> Result<(), String> {
		let data = self.data.borrow_mut();

		unsafe {
			libc::mprotect(
				data.code_buffer_pointer.as_ptr().cast::<libc::c_void>(),
				data.fixed_capacity_in_bytes,
				libc::PROT_READ | libc::PROT_EXEC,
			);

			libc::mprotect(
				data.data_buffer_pointer.as_ptr().cast::<libc::c_void>(),
				data.fixed_capacity_in_bytes,
				libc::PROT_READ | libc::PROT_WRITE,
			);
		}

		Ok(())
	}

	fn destroy(&mut self) {
		let data = self.data.borrow_mut();

		unsafe {
			libc::munmap(
				data.code_buffer_pointer.as_ptr().cast::<libc::c_void>(),
				data.fixed_capacity_in_bytes,
			);

			libc::munmap(
				data.data_buffer_pointer.as_ptr().cast::<libc::c_void>(),
				data.fixed_capacity_in_bytes,
			);
		}
	}
}

#[derive(Debug)]
struct MemoryManagerData {
	fixed_capacity_in_bytes: usize,
	fixed_page_size: usize,
	code_buffer_pointer: NonNull<u8>,
	code_offset: usize,
	data_buffer_pointer: NonNull<u8>,
	data_offset: usize,
}
