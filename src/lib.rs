extern crate bigint;
extern crate libc;

use std::mem::transmute;

pub mod ffi;
pub mod types;
pub mod utils;

use libc::c_void;
use bigint::uint::U256;
use ffi::{randomx_flags, randomx_calculate_hash, randomx_vm};

pub use types::RxState;

pub fn calculate(vm: *mut randomx_vm, input: &mut [u8], nonce: u64) -> U256 {
	let mut result: [u8; 32] = [0; 32];
	let nonce_bytes: [u8; 8] = unsafe { transmute(nonce.to_be()) };
	let length = input.len();

	for i in 0..8 {
		input[length - (8-i)] = nonce_bytes[i];
	}

	unsafe {
		randomx_calculate_hash(
			vm,
			input.as_ptr() as *const c_void,
			length,
			result.as_mut_ptr() as *mut c_void);
	}

	result.into()
} 

pub fn slow_hash(state: &mut RxState, data: &[u8], seed: &[u8; 32]) -> U256 {
	let flags: randomx_flags = ffi::randomx_flags_RANDOMX_FLAG_DEFAULT;

	let hash_target = unsafe {
		let mut hash: [u8; 32] = [0; 32];

		let cache = state.init_cache(seed, true).expect("seed no initialized");
		let vm = state.create_vm().expect("vm no initialized");

		ffi::randomx_calculate_hash(
			vm,
			data.as_ptr() as *const c_void,
			data.len(),
			hash.as_mut_ptr() as *mut c_void,
		);

		ffi::randomx_destroy_vm(vm);

		state.destroy();

		hash.into()
	};

	hash_target
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_verify() {
		let hash = [
			3445087034, 2648376634, 1922307026, 2053423067, 1138766641, 1228060755, 3862626153,
			4108850211,
		];
		let hash_u256: U256 = from_u32_to_U256(&hash);
		let block_template: [u8; 128] = [0; 128];
		let seed: [u8; 32] = [0; 32];

		let mut rx_state = RxState::new();

		assert_eq!(hash_u256, slow_hash(&mut rx_state, &block_template, &seed));
	}
}
