extern crate libc;

use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use std::thread;
use std::ptr::NonNull;

use bigint::uint::U256;
use libc::c_void;
use ffi::*;

struct Wrapper<T>(NonNull<T>);
unsafe impl<T> std::marker::Send for Wrapper<T> { }

pub type RxCache = Option<*mut randomx_cache>;
pub type RxDataset = Option<*mut randomx_dataset>;

#[derive(Debug, Clone)]
pub struct RxState {
	pub seed: u64,
	pub hard_aes: bool,
	pub full_mem: bool,
	pub large_pages: bool,
	pub jit_compiler: bool,
	cache: RxCache,
	dataset: RxDataset,
}

unsafe impl Sync for RxState{}
unsafe impl Send for RxState{}

impl RxState {
	pub fn new() -> Self {
		RxState {
			seed: 0,
			hard_aes: false,
			full_mem: false,
			large_pages: false,
			jit_compiler: false,
			cache: None,
			dataset: None,
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

	pub unsafe fn init_cache(&mut self, seed: &[u8], reinit: bool) -> Result<(), &str> {
		if let Some(c) = self.cache {
			if !reinit {
				randomx_release_cache(c);
			} else {
				return Ok(());
			}
		}

		let flags = self.get_flags();
		let mut cache = randomx_alloc_cache(flags | randomx_flags_RANDOMX_FLAG_LARGE_PAGES);

		if cache.is_null() {
			cache = randomx_alloc_cache(flags);

			if cache.is_null() {
				return Err("cache no allocated");
			}
		}

		randomx_init_cache(cache, seed.as_ptr() as *const c_void, seed.len());

		//forget(cache);
		self.cache = Some(cache);

		Ok(())
	}

	pub unsafe fn init_dataset(&mut self, threads_count: u8) -> Result<(), &str> {
		if let Some(_) = self.dataset {
			return Ok(());
		}

		let cache = match self.cache {
			Some(c) => c,
			None => {
				return Err("cache is not initialized");
			}
		};

		let mut dataset = randomx_alloc_dataset(randomx_flags_RANDOMX_FLAG_LARGE_PAGES);

		if dataset.is_null() {
			dataset = randomx_alloc_dataset(self.get_flags());
		}

		if dataset.is_null() {
			return Err("is not possible initialize a dataset");
		}

		let mut threads = Vec::new();
		let mut start = 0;
		let count = randomx_dataset_item_count();
		let perth = count / threads_count as u64;
		let remainder = count % threads_count as u64;

		for i in 0..threads_count {
			let cache = Wrapper(NonNull::new(cache).unwrap());
			let dataset = Wrapper(NonNull::new(dataset).unwrap());
			let count = perth + if i == (threads_count - 1) { remainder } else {0};
			threads.push(thread::spawn(move || {
				let d = dataset.0.as_ptr();
				let c = cache.0.as_ptr();
				randomx_init_dataset(d, c, start, count);
			}));
			start += count;
		}

		for th in threads {
			th.join();
		}

		self.dataset = Some(dataset);

		Ok(())
	}

	pub unsafe fn create_vm(&mut self) -> Result<*mut randomx_vm, &str> {
		let cache = match self.cache {
			Some(c) => c,
			None => {
				return Err("cache is not initialized");
			}
		};

		let dataset = match self.dataset {
			Some(d) => d,
			None => {null_mut()}
		};

		let flags = self.get_flags()
			| if !dataset.is_null() {
				randomx_flags_RANDOMX_FLAG_FULL_MEM
			} else {
				0
			};

		let mut vm = randomx_create_vm(
			flags | randomx_flags_RANDOMX_FLAG_LARGE_PAGES, cache, dataset);

		if vm.is_null() {
			vm = randomx_create_vm(flags, cache, dataset);
		}

		if vm.is_null() {
			vm = randomx_create_vm(randomx_flags_RANDOMX_FLAG_DEFAULT, cache, dataset);
		}

		if !vm.is_null() {
			Ok(vm)
		} else {
			Err("unable")
		}
	}

	pub unsafe fn destroy(&mut self) {
		/*if !self.cache.is_null() {
			randomx_release_cache(self.cache);
			self.cache = null_mut();
		}

		if !self.dataset.is_null() {
			randomx_release_dataset(self.dataset);
			self.dataset = null_mut();
		}*/
	}
}
