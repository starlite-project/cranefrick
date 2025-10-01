use std::{
	alloc::{self, Layout},
	mem, ptr,
};

use inkwell::memory_manager::McjitMemoryManager;

#[derive(Debug, Default)]
pub struct MemoryManager {
	allocs: Vec<MemoryRecord>,
}

impl MemoryManager {
	fn alloc_memory_segment(
		&mut self,
		ptr: *mut libc::c_void,
		layout: Layout,
		readable: bool,
		writable: bool,
		executable: bool,
	) -> *mut u8 {
		let mut prot = 0;

		if readable {
			prot |= libc::PROT_READ;
		}

		if writable {
			prot |= libc::PROT_WRITE;
		}

		if executable {
			prot |= libc::PROT_EXEC;
		}

		let ptr = unsafe {
			libc::mmap(
				ptr,
				layout.size(),
				prot,
				libc::MAP_PRIVATE | libc::MAP_ANON,
				-1,
				0,
			)
		};

		if ptr::eq(ptr, libc::MAP_FAILED) {
			return ptr::null_mut();
		}

		let ptr = ptr.cast::<u8>();
		self.allocs.push(MemoryRecord { ptr, layout });

		ptr
	}
}

impl McjitMemoryManager for MemoryManager {
	#[tracing::instrument(skip(self))]
	fn allocate_code_section(
		&mut self,
		size: libc::uintptr_t,
		alignment: libc::c_uint,
		section_id: libc::c_uint,
		section_name: &str,
	) -> *mut u8 {
		let layout = unsafe { Layout::from_size_align_unchecked(size, alignment as usize) };

		tracing::trace!(layout = ?layout);

		let ptr = unsafe { alloc::alloc(layout) };

		self.alloc_memory_segment(ptr.cast(), layout, true, true, true)
	}

	#[tracing::instrument(skip(self))]
	fn allocate_data_section(
		&mut self,
		size: libc::uintptr_t,
		alignment: libc::c_uint,
		section_id: libc::c_uint,
		section_name: &str,
		is_read_only: bool,
	) -> *mut u8 {
		let layout = unsafe { Layout::from_size_align_unchecked(size, alignment as usize) };

		tracing::trace!(layout = ?layout);

		let ptr = unsafe { alloc::alloc(layout) };

		self.alloc_memory_segment(ptr.cast(), layout, true, !is_read_only, false)
	}

	fn destroy(&mut self) {
		for record in mem::take(&mut self.allocs) {
			unsafe { alloc::dealloc(record.ptr, record.layout) }
		}
	}

	fn finalize_memory(&mut self) -> Result<(), String> {
		Ok(())
	}
}

#[derive(Debug)]
struct MemoryRecord {
	ptr: *mut u8,
	layout: Layout,
}
