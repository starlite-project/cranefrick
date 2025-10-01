use std::{
	alloc::{self, Layout},
	mem,
	ptr::{self, NonNull},
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
		writable: bool,
		executable: bool,
		section_name: String,
	) -> *mut u8 {
		let mut prot = libc::PROT_READ;

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

		let Some(record) = MemoryRecord::new(ptr, layout, executable, section_name) else {
			return ptr::null_mut();
		};

		self.allocs.push(record);

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

		self.alloc_memory_segment(ptr.cast(), layout, true, true, section_name.to_owned())
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

		self.alloc_memory_segment(
			ptr.cast(),
			layout,
			!is_read_only,
			false,
			section_name.to_owned(),
		)
	}

	#[tracing::instrument]
	fn destroy(&mut self) {
		tracing::trace!("deallocating memory");

		for record in mem::take(&mut self.allocs) {
			unsafe { libc::munmap(record.ptr.as_ptr().cast(), record.layout.size()) };

			unsafe { alloc::dealloc(record.ptr.as_ptr(), record.layout) }
		}
	}

	#[tracing::instrument]
	fn finalize_memory(&mut self) -> Result<(), String> {
		tracing::trace!("finalizing memory");

		for record in &self.allocs {
			let mut prot = libc::PROT_READ;

			if record.executable {
				prot |= libc::PROT_EXEC;
			}

			let res =
				unsafe { libc::mprotect(record.ptr.as_ptr().cast(), record.layout.size(), prot) };

			if !matches!(res, 0) {
				return Err(format!(
					"failed to protect memory for section {}",
					record.section_name
				));
			}
		}

		Ok(())
	}
}

#[derive(Debug)]
struct MemoryRecord {
	ptr: NonNull<u8>,
	layout: Layout,
	executable: bool,
	section_name: String,
}

impl MemoryRecord {
	pub fn new(
		ptr: *mut u8,
		layout: Layout,
		executable: bool,
		section_name: String,
	) -> Option<Self> {
		Some(Self {
			ptr: NonNull::new(ptr)?,
			layout,
			executable,
			section_name,
		})
	}
}
