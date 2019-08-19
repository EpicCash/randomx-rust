extern crate libc;

use std::ptr::null_mut;
use std::ptr::NonNull;
use std::sync::Arc;
use std::thread;

use ffi::*;
use libc::c_void;

struct Wrapper<T>(NonNull<T>);
unsafe impl<T> std::marker::Send for Wrapper<T> {}

pub enum RxAction {
	Changed,
	NotChanged,
}

#[derive(Debug, Clone)]
pub struct Trash {
	cache: Option<RxCache>,
	dataset: Option<RxDataset>,
}

impl Trash {
	pub fn empty(&mut self) {
		self.cache = None;
		self.dataset = None;
	}
}

impl Default for Trash {
	fn default() -> Self {
		Trash {
			cache: None,
			dataset: None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct RxCache {
	cache: *mut randomx_cache,
}

impl Drop for RxCache {
	fn drop(&mut self) {
		unsafe {
			randomx_release_cache(self.cache);
		}
	}
}

#[derive(Debug, Clone)]
pub struct RxDataset {
	dataset: *mut randomx_dataset,
}

impl Drop for RxDataset {
	fn drop(&mut self) {
		unsafe {
			randomx_release_dataset(self.dataset);
		}
	}
}

#[derive(Debug, Clone)]
pub struct RxState {
	pub seed: [u8; 32],
	pub hard_aes: bool,
	pub full_mem: bool,
	pub large_pages: bool,
	pub jit_compiler: bool,
	cache: Option<RxCache>,
	dataset: Option<RxDataset>,
	vms: Vec<Arc<RxVM>>,
	trash: Trash,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RxVM {
	pub vm: *mut randomx_vm,
}

impl Drop for RxVM {
	fn drop(&mut self) {
		unsafe {
			randomx_destroy_vm(self.vm);
		}
	}
}

unsafe impl Sync for RxState {}
unsafe impl Send for RxState {}

impl RxState {
	pub fn new() -> Self {
		RxState {
			seed: [0; 32],
			hard_aes: false,
			full_mem: false,
			large_pages: false,
			jit_compiler: false,
			cache: None,
			dataset: None,
			vms: vec![],
			trash: Trash::default(),
		}
	}

	pub fn is_initialized(&self) -> bool {
		if let Some(_) = self.cache {
			true
		} else {
			false
		}
	}

	pub fn get_flags(&self) -> randomx_flags {
		let mut flags = randomx_flags_RANDOMX_FLAG_DEFAULT;

		if self.jit_compiler {
			flags |= randomx_flags_RANDOMX_FLAG_JIT;
		}

		if self.hard_aes {
			flags |= randomx_flags_RANDOMX_FLAG_HARD_AES
		}

		if self.full_mem {
			flags |= randomx_flags_RANDOMX_FLAG_FULL_MEM;
		}

		if self.large_pages {
			flags |= randomx_flags_RANDOMX_FLAG_LARGE_PAGES;
		}

		flags
	}

	pub fn init_cache(&mut self, seed: &[u8]) -> Result<RxAction, &str> {
		if let Some(_) = self.cache {
			if self.is_same_seed(seed) {
				return Ok(RxAction::NotChanged);
			}
		}

		let flags = self.get_flags();
		let mut cache_ptr =
			unsafe { randomx_alloc_cache(flags | randomx_flags_RANDOMX_FLAG_LARGE_PAGES) };

		if cache_ptr.is_null() {
			cache_ptr = unsafe { randomx_alloc_cache(flags) };
		}

		if cache_ptr.is_null() {
			return Err("cache not allocated");
		}

		unsafe {
			randomx_init_cache(cache_ptr, seed.as_ptr() as *const c_void, seed.len());
		}

		self.trash.cache = self.cache.take();
		self.cache = Some(RxCache { cache: cache_ptr });
		self.seed.copy_from_slice(seed);

		Ok(RxAction::Changed)
	}

	pub fn is_same_seed(&self, seed: &[u8]) -> bool {
		&self.seed == seed
	}

	pub fn init_dataset(&mut self, threads_count: u8) -> Result<(), &str> {
		let cache = self.cache.as_ref().ok_or("cache is not initialized")?;

		//let mut dataset_ptr =
		//	unsafe { randomx_alloc_dataset(randomx_flags_RANDOMX_FLAG_LARGE_PAGES) };
		let mut dataset_ptr = unsafe { randomx_alloc_dataset(self.get_flags()) };

		/*if dataset_ptr.is_null() {
			dataset_ptr = unsafe { randomx_alloc_dataset(self.get_flags()) };
		}*/

		if dataset_ptr.is_null() {
			return Err("it's not possible initialize a dataset");
		}

		let mut threads = Vec::new();
		let mut start: u64 = 0;
		let count: u64 = unsafe { randomx_dataset_item_count() } as u64;
		let perth: u64 = count / threads_count as u64;
		let remainder: u64 = count % threads_count as u64;

		for i in 0..threads_count {
			let cache = Wrapper(NonNull::new(cache.cache).unwrap());
			let dataset = Wrapper(NonNull::new(dataset_ptr).unwrap());
			let count = perth
				+ if i == (threads_count - 1) {
					remainder
				} else {
					0
				};
			threads.push(thread::spawn(move || {
				let d = dataset.0.as_ptr();
				let c = cache.0.as_ptr();
				unsafe {
					randomx_init_dataset(d, c, start.into(), count.into());
				}
			}));
			start += count;
		}

		for th in threads {
			th.join().map_err(|_| "failed to join threads")?;
		}

		self.trash.dataset = self.dataset.take();
		self.dataset = Some(RxDataset {
			dataset: dataset_ptr,
		});

		Ok(())
	}

	pub fn create_vm(&mut self) -> Result<Arc<RxVM>, &str> {
		let cache = self.cache.as_ref().ok_or("cache is not initialized")?;

		let dataset = self
			.dataset
			.as_ref()
			.map(|d| d.dataset)
			.unwrap_or(null_mut());

		let flags = self.get_flags()
			| if !dataset.is_null() {
				randomx_flags_RANDOMX_FLAG_FULL_MEM
			} else {
				0
			};

		let mut vm = unsafe {
			randomx_create_vm(
				flags | randomx_flags_RANDOMX_FLAG_LARGE_PAGES,
				cache.cache,
				dataset,
			)
		};

		if vm.is_null() {
			vm = unsafe { randomx_create_vm(flags, cache.cache, dataset) };
		}

		if vm.is_null() {
			vm = unsafe {
				randomx_create_vm(randomx_flags_RANDOMX_FLAG_DEFAULT, cache.cache, dataset)
			};
		}

		if !vm.is_null() {
			self.vms.push(Arc::new(RxVM { vm }));
			Ok(self.vms.last().unwrap().clone())
		} else {
			Err("unable to create RxVM")
		}
	}

	pub fn get_or_create_vm(&mut self) -> Result<Arc<RxVM>, &str> {
		if self.vms.len() == 0 {
			self.create_vm()
		} else {
			Ok(self.vms.last().unwrap().clone())
		}
	}

	pub fn update_vms(&mut self) {
		let cache = self.cache.as_ref().map_or(null_mut(), |x| x.cache);
		let dataset = self.dataset.as_ref().map_or(null_mut(), |x| x.dataset);

		for vm in &self.vms {
			unsafe {
				randomx_vm_set_cache(vm.vm, cache);
				randomx_vm_set_dataset(vm.vm, dataset);
			}
		}

		self.trash.empty();
	}
}
