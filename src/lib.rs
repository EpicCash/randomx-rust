extern crate bigint;
extern crate byteorder;
extern crate libc;

pub mod ffi;
pub mod types;
pub mod utils;

use bigint::uint::U256;
use byteorder::{BigEndian, ByteOrder};
use libc::c_void;

use ffi::randomx_calculate_hash;

pub use types::{RxAction, RxState, RxVM};

pub fn calculate(vm: &RxVM, input: &mut [u8], nonce: u64) -> U256 {
	let mut result: [u8; 32] = [0; 32];
	let input_size = input.len();

	let mut nonce_bytes = [0; 8];
	BigEndian::write_u64(&mut nonce_bytes, nonce);

	for i in 0..nonce_bytes.len() {
		input[input_size - (nonce_bytes.len() - i)] = nonce_bytes[i];
	}

	unsafe {
		randomx_calculate_hash(
			vm.vm,
			input.as_ptr() as *const c_void,
			input_size as u64,
			result.as_mut_ptr() as *mut c_void,
		);
	}

	result.into()
}

pub fn slow_hash(state: &mut RxState, data: &[u8], seed: &[u8; 32]) -> U256 {
	let vm = {
		state.jit_compiler = true;
		if let RxAction::Changed = state.init_cache(seed).unwrap() {
			state.update_vms();
		}
		state.get_or_create_vm().expect("vm not initialized")
	};

	let hash_target = unsafe {
		let mut hash: [u8; 32] = [0; 32];

		ffi::randomx_calculate_hash(
			vm.read().unwrap().vm,
			data.as_ptr() as *const c_void,
			data.len() as u64,
			hash.as_mut_ptr() as *mut c_void,
		);

		hash.into()
	};

	hash_target
}

#[cfg(test)]
mod test {

	use super::*;

	#[test]
	fn test_verify() {
		let hash: U256 = [
			58, 219, 87, 205, 58, 5, 219, 157, 210, 19, 148, 114, 219, 191, 100, 122, 49, 51, 224,
			67, 83, 184, 50, 73, 105, 255, 58, 230, 35, 20, 232, 244,
		]
		.into();
		let block_template: [u8; 128] = [0; 128];
		let seed: [u8; 32] = [0; 32];

		let mut rx_state = RxState::new();

		assert_eq!(hash, slow_hash(&mut rx_state, &block_template, &seed));
	}

	#[test]
	#[ignore]
	fn test_swap_dataset() {
		let hashs = vec![
			U256::from_dec_str(
				"26621690709847676946322902081806750977287422934645095895756323911047673342196",
			)
			.unwrap(),
			U256::from_dec_str(
				"99798341874875334058428891982218724161246716553034279961270815837069075885600",
			)
			.unwrap(),
		];

		let mut block_template: [u8; 128] = [0; 128];
		let mut rx = RxState::new();

		rx.full_mem = true;
		rx.jit_compiler = true;

		rx.init_cache(&[0u8; 32])
			.expect("Is not possible initialize the cache!");

		rx.init_dataset(1)
			.expect("Is not possible initialize the dataset");

		let vm_lock = rx.get_or_create_vm().unwrap();

		let vm = vm_lock.read().unwrap();

		let hash = calculate(&vm, &mut block_template, 0);

		assert_eq!(hash, hashs[0]);

		rx.init_cache(&[20u8; 32])
			.expect("Is not possible initialize the cache!");

		rx.init_dataset(1)
			.expect("Is not possible initialize the dataset");

		rx.update_vms();

		let mut block_template: [u8; 128] = [0; 128];
		let hash2 = calculate(&vm, &mut block_template, 0);

		assert_eq!(hash2, hashs[1]);
	}
}
