extern crate libc;

use std::ptr::null_mut;
use std::thread;
use std::ptr::NonNull;

use libc::c_void;
use ffi::*;

struct Wrapper<T>(NonNull<T>);
unsafe impl<T> std::marker::Send for Wrapper<T> { }

#[derive(Debug)]
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct RxState {
	pub seed: u64,
	pub hard_aes: bool,
	pub full_mem: bool,
	pub large_pages: bool,
	pub jit_compiler: bool,
	cache: Option<RxCache>,
	dataset: Option<RxDataset>,
}

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

	pub fn init_cache(&mut self, seed: &[u8], reinit: bool) -> Result<(), &str> {
		if let Some(ref c) = self.cache {
			if !reinit {
                unsafe {
				    randomx_release_cache(c.cache);
                }
			} else {
				return Ok(());
			}
		}

		let flags = self.get_flags();
		let mut cache = unsafe {randomx_alloc_cache(flags | randomx_flags_RANDOMX_FLAG_LARGE_PAGES) };

		if cache.is_null() {
			cache = unsafe { randomx_alloc_cache(flags) };

			if cache.is_null() {
				return Err("cache no allocated");
			}
		}

        unsafe {
		    randomx_init_cache(cache, seed.as_ptr() as *const c_void, seed.len());
        }

		//forget(cache);
		self.cache = Some(RxCache { cache });

		Ok(())
	}

	pub fn init_dataset(&mut self, threads_count: u8) -> Result<(), &str> {
		if let Some(_) = self.dataset {
			return Ok(());
		}

		let cache = match self.cache {
			Some(ref c) => c,
			None => {
				return Err("cache is not initialized");
			}
		};

		let mut dataset = unsafe {
            randomx_alloc_dataset(randomx_flags_RANDOMX_FLAG_LARGE_PAGES)
        };

		if dataset.is_null() {
			dataset = unsafe { randomx_alloc_dataset(self.get_flags()) };
		}

		if dataset.is_null() {
			return Err("is not possible initialize a dataset");
		}

		let mut threads = Vec::new();
		let mut start = 0;
		let count = unsafe { randomx_dataset_item_count() };
		let perth = count / threads_count as u64;
		let remainder = count % threads_count as u64;

		for i in 0..threads_count {
			let cache = Wrapper(NonNull::new(cache.cache).unwrap());
			let dataset = Wrapper(NonNull::new(dataset).unwrap());
			let count = perth + if i == (threads_count - 1) { remainder } else {0};
			threads.push(thread::spawn(move || {
				let d = dataset.0.as_ptr();
				let c = cache.0.as_ptr();
                unsafe {
				    randomx_init_dataset(d, c, start, count);
                }
			}));
			start += count;
		}

		for th in threads {
			th.join().map_err(|_| "failed to join threads")?;
		}

		self.dataset = Some(RxDataset { dataset });

		Ok(())
	}

	pub fn create_vm(&mut self) -> Result<RxVM, &str> {
		let cache = match self.cache {
			Some(ref c) => c,
			None => {
				return Err("cache is not initialized");
			}
		};

		let dataset = match self.dataset {
			Some(ref d) => d.dataset,
			None => {null_mut()}
		};

		let flags = self.get_flags()
			| if !dataset.is_null() {
				randomx_flags_RANDOMX_FLAG_FULL_MEM
			} else {
				0
			};

		let mut vm = unsafe {
            randomx_create_vm(
			flags | randomx_flags_RANDOMX_FLAG_LARGE_PAGES, cache.cache, dataset)
        };

		if vm.is_null() {
			vm = unsafe { randomx_create_vm(flags, cache.cache, dataset) };
		}

		if vm.is_null() {
			vm = unsafe {randomx_create_vm(randomx_flags_RANDOMX_FLAG_DEFAULT, cache.cache, dataset) };
		}

		if !vm.is_null() {
			Ok(RxVM { vm })
		} else {
			Err("unable")
		}
	}
}
