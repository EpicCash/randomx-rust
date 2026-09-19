extern crate byteorder;
extern crate libc;

pub mod ffi;
pub mod types;
pub mod utils;

use byteorder::{BigEndian, ByteOrder};
use libc::c_void;

use ffi::randomx_calculate_hash;

pub use types::{RxAction, RxState, RxVM};

pub fn calculate(vm: &RxVM, input: &mut [u8], nonce: u64) -> [u8; 32] {
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

    result
}

pub fn slow_hash(state: &mut RxState, data: &[u8], seed: &[u8; 32]) -> [u8; 32] {
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

        hash
    };

    hash_target
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_verify() {
        let expected_hash: [u8; 32] = [
            58, 219, 87, 205, 58, 5, 219, 157,
            210, 19, 148, 114, 219, 191, 100, 122,
            49, 51, 224, 67, 83, 184, 50, 73,
            105, 255, 58, 230, 35, 20, 232, 244,
        ];

        let block_template: [u8; 128] = [0; 128];
        let seed: [u8; 32] = [0; 32];

        let mut rx_state = RxState::new();

        assert_eq!(
            expected_hash,
            slow_hash(&mut rx_state, &block_template, &seed)
        );
    }

    #[test]
    #[ignore]
    fn test_swap_dataset() {
        let expected_hashes: [[u8; 32]; 2] = [
            [
                58, 219, 87, 205, 58, 5, 219, 157,
                210, 19, 148, 114, 219, 191, 100, 122,
                49, 51, 224, 67, 83, 184, 50, 73,
                105, 255, 58, 230, 35, 20, 232, 244,
            ],
            [
                220, 163, 220, 27, 27, 96, 46, 20,
                177, 168, 125, 133, 170, 72, 202, 19,
                144, 175, 112, 19, 186, 92, 60, 49,
                65, 196, 178, 174, 177, 245, 46, 32,
            ],
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

        {
            let vm = vm_lock.read().unwrap();
            let hash = calculate(&vm, &mut block_template, 0);

            assert_eq!(hash, expected_hashes[0]);
        }

        rx.init_cache(&[20u8; 32])
            .expect("Is not possible initialize the cache!");

        rx.init_dataset(1)
            .expect("Is not possible initialize the dataset");

        rx.update_vms();

        let mut block_template: [u8; 128] = [0; 128];

        {
            let vm = vm_lock.read().unwrap();
            let hash2 = calculate(&vm, &mut block_template, 0);

            assert_eq!(hash2, expected_hashes[1]);
        }
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
        rx_state.full_mem = false; // Use the default interpreter

        // Initialize cache with the seed
        rx_state.init_cache(&seed).expect("Failed to init cache");

        // Create VM
        let vm = rx_state.get_or_create_vm().expect("Failed to create VM");

        // Prepare output buffer
        let mut input_buf = [0u8; 128];
        input_buf[..input.len()].copy_from_slice(input);

        // Hash
        let hash = calculate(&vm.read().unwrap(), &mut input_buf, 0);

        // Print [u8; 32] as hexadecimal.
        let hash_hex: String = hash
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect();

        println!("RandomX hash: {}", hash_hex);

        // If adding a known-value assertion later:
        //
        // let expected_hash: [u8; 32] = [
        //     ...
        // ];
        // assert_eq!(hash, expected_hash);
    }
}
