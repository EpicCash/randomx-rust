extern crate bigint;
extern crate libc;
extern crate byteorder;

pub mod ffi;
pub mod types;
pub mod utils;

use libc::c_void;
use bigint::uint::U256;
use byteorder::{BigEndian, ByteOrder};

use ffi::{randomx_flags, randomx_calculate_hash, randomx_vm};

pub use types::RxState;

pub fn calculate(vm: *mut randomx_vm, input: &mut [u8], nonce: u64) -> U256 {
	let mut result: [u8; 32] = [0; 32];
	let input_size = input.len();

	let mut nonce_bytes = [0; 8];
	BigEndian::write_u64(&mut nonce_bytes, nonce);

	// first example
	for i in 0..nonce_bytes.len(){
		input[input_size - (nonce_bytes.len()-i)] = nonce_bytes[i];
	}

	// after test it
	// let mut s_input: Vec<u8> = input.into_iter()
	//	.take(input_size - 8)
	//	.chain(&mut nonce_bytes)
	//	.collect::<Vec<u8>>();

	unsafe {
		randomx_calculate_hash(
			vm,
			input.as_ptr() as *const c_void,
			input_size,
			result.as_mut_ptr() as *mut c_void);
	}

	result.into()
} 

pub fn slow_hash(state: &mut RxState, data: &[u8], seed: &[u8; 32]) -> U256 {
	let vm = unsafe {
		let cache = state.init_cache(seed, false).expect("seed no initialized");
		state.create_vm().expect("vm no initialized")
	};

	let hash_target = unsafe {
		let mut hash: [u8; 32] = [0; 32];

		ffi::randomx_calculate_hash(
			vm,
			data.as_ptr() as *const c_void,
			data.len(),
			hash.as_mut_ptr() as *mut c_void,
		);

		ffi::randomx_destroy_vm(vm);

		hash.into()
	};

	hash_target
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::utils::*;

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
