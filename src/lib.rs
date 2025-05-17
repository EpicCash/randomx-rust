extern crate byteorder;
extern crate libc;

pub mod ffi;
pub mod types;
pub mod utils;

use byteorder::{BigEndian, ByteOrder};
use libc::c_void;

use num_bigint::BigUint;


use ffi::randomx_calculate_hash;

pub use types::{RxAction, RxState, RxVM};

pub fn calculate(vm: &RxVM, input: &mut [u8], nonce: u64) -> BigUint {
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
            input_size,
            result.as_mut_ptr() as *mut c_void,
        );
    }

    BigUint::from_bytes_be(&result)
}

pub fn slow_hash(state: &mut RxState, data: &[u8], seed: &[u8; 32]) -> BigUint {
    // Only reinitialize cache if the seed changes
    if let RxAction::Changed = state.init_cache(seed).unwrap() {
        // If full_mem is true, also reinitialize dataset
        if state.full_mem {
            state.init_dataset(1).expect("Failed to init dataset");
        }
        state.update_vms();
    }

    // Use the VM as configured in state
    let vm = state.get_or_create_vm().expect("vm not initialized");

    let hash_target = unsafe {
        let mut hash: [u8; 32] = [0; 32];

        ffi::randomx_calculate_hash(
            vm.read().unwrap().vm,
            data.as_ptr() as *const c_void,
            data.len(),
            hash.as_mut_ptr() as *mut c_void,
        );

        BigUint::from_bytes_be(&hash)
    };

    hash_target
}

#[cfg(test)]
mod test {
    use super::*;
    use num_bigint::BigUint;
    use std::str::FromStr;
    #[test]
    fn test_verify() {
        let hash = BigUint::from_bytes_be(&[
            58, 219, 87, 205, 58, 5, 219, 157, 210, 19, 148, 114, 219, 191, 100, 122, 49, 51, 224,
            67, 83, 184, 50, 73, 105, 255, 58, 230, 35, 20, 232, 244,
        ]);
        let block_template: [u8; 128] = [0; 128];
        let seed: [u8; 32] = [0; 32];

        let mut rx_state = RxState::new();
        
       

        assert_eq!(hash, slow_hash(&mut rx_state, &block_template, &seed));
    }

    #[test]
    #[ignore]
    fn test_swap_dataset() {
        let hashs = vec![
            BigUint::from_str(
                "26621690709847676946322902081806750977287422934645095895756323911047673342196"
                
            )
            .unwrap(),
            BigUint::from_str(
                "99798341874875334058428891982218724161246716553034279961270815837069075885600"
                
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

    #[test]
    fn test_randomx_simple_hash() {

        // Example input and seed
        let input = b"Hello, RandomX!";
        let seed: [u8; 32] = [1; 32];

        // Prepare state
        let mut rx_state = RxState::new();
        rx_state.hard_aes = true; // Important for Apple Silicon/ARM!
        rx_state.jit_compiler = false;
        rx_state.full_mem = false;// Use the default interpreter

        // Initialize cache with the seed
        rx_state.init_cache(&seed).expect("Failed to init cache");

        // Create VM
        let vm = rx_state.get_or_create_vm().expect("Failed to create VM");

        // Prepare output buffer
        let mut input_buf = [0u8; 128];
        input_buf[..input.len()].copy_from_slice(input);

        // Hash
        let hash = calculate(&vm.read().unwrap(), &mut input_buf, 0);

        // Print hash as hex
        println!("RandomX hash: {:x}", hash);

        // Optionally, check against a known value (if you have one)
        // assert_eq!(hash, BigUint::from_bytes_be(&hex!("...")));
    }

   
}
