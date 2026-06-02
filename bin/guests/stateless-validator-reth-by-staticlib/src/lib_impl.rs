#[cfg(zisk_guest)]
extern crate alloc;

#[cfg(zisk_guest)]
use alloc::sync::Arc;
#[cfg(not(zisk_guest))]
use std::sync::Arc;

use alloy_consensus::crypto::install_default_provider;
use revm::install_crypto;

use guest_reth::{
    CustomEvmCrypto, RethInputPublic, RethInputWitness, extract_block_info, get_chain_name,
    get_chain_spec, validate_block_stateless, verify_signatures,
};

// Panic handler for bare-metal targets. The zkvm target gets its panic handler
// from libziskos_staticlib.a; on other guest targets we provide one here.
#[cfg(all(target_arch = "riscv64", target_os = "none"))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Provided by libziskos_staticlib.a on all targets.
unsafe extern "C" {
    fn zkvm_init();
    fn zkvm_deinit();
    fn read_input(buf_ptr: *mut *const u8, buf_size: *mut usize);
    fn write_output(output: *const u8, size: usize);
}

// sys_write is only compiled into libziskos_staticlib.a for guest targets.
#[cfg(zisk_guest)]
unsafe extern "C" {
    fn sys_write(fd: u32, write_buf: *const u8, nbytes: usize);
}

// println-like macro that works on all three targets:
//   zisk_guest (zkvm + bare-metal riscv64im): formats via alloc::format!, writes via sys_write
//   host (std): delegates to std println!
macro_rules! zisk_println {
    ($($arg:tt)*) => {{
        #[cfg(zisk_guest)]
        {
            let s = alloc::format!($($arg)*);
            unsafe {
                sys_write(1, s.as_ptr(), s.len());
                sys_write(1, b"\n".as_ptr(), 1);
            }
        }
        #[cfg(not(zisk_guest))]
        println!($($arg)*);
    }};
}

// Bump allocator from libziskos_staticlib.a — only available on the zkVM target.
#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
unsafe extern "C" {
    fn sys_alloc_aligned(bytes: usize, align: usize) -> *mut u8;
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
use core::alloc::{GlobalAlloc, Layout};

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
struct ZiskAllocator;

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
unsafe impl GlobalAlloc for ZiskAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { sys_alloc_aligned(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}

    // The zkVM heap is pre-zeroed, so allocating is sufficient.
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { sys_alloc_aligned(layout.size(), layout.align()) }
    }
}

#[cfg(all(target_os = "zkvm", target_vendor = "zisk"))]
#[global_allocator]
static ALLOC: ZiskAllocator = ZiskAllocator;

// Bare-metal riscv64im: standalone bump allocator backed by linker-script heap symbols.
#[cfg(all(target_arch = "riscv64", target_os = "none"))]
mod bump_alloc;

#[unsafe(no_mangle)]
pub fn main() {
    unsafe { zkvm_init() };

    // Initialize the bump allocator from linker-script heap bounds (riscv64im only).
    #[cfg(all(target_arch = "riscv64", target_os = "none"))]
    unsafe { bump_alloc::init_heap() };

    // Install ZisK-accelerated EVM crypto (all guest targets).
    install_crypto(CustomEvmCrypto::default());
    install_default_provider(Arc::new(CustomEvmCrypto::default())).unwrap();

    // read_input returns the full combined buffer, which contains two length-prefixed
    // sub-records: [8B len1][pub_bytes][pad][8B len2][wit_bytes].
    let mut ptr: *const u8;
    unsafe {
        let mut p: *const u8 = core::ptr::null();
        let mut s: usize = 0;
        read_input(&mut p, &mut s);
        ptr = p;
    }

    // Parse first sub-record: public input
    let public: RethInputPublic = unsafe {
        let sub_len =
            u64::from_le_bytes(core::slice::from_raw_parts(ptr, 8).try_into().unwrap()) as usize;
        ptr = ptr.add(8);
        let val = bincode::serde::decode_from_slice(
            core::slice::from_raw_parts(ptr, sub_len),
            bincode::config::standard(),
        )
        .map(|(v, _)| v)
        .expect("Deserialization failed");
        ptr = ptr.add((sub_len + 7) & !7);
        val
    };

    // Get chain config
    let chain_config = public.chain_config().clone();

    // Extract block info for logging.
    let block = public.block().clone();
    let (block_number, gas_used, tx_count) = extract_block_info(&block);
    let chain_id = chain_config.chain_id;
    let chain = get_chain_name(chain_id);
    zisk_println!("Executing block validation for {} Block #{} ({} txs)", chain, block_number, tx_count);

    // Verify signatures
    let chain_spec = get_chain_spec(&chain_config);
    let block = verify_signatures(block, chain_spec.clone(), public.public_keys)
        .expect("Signature verification failed");

    // Parse second sub-record: witness
    let witness: RethInputWitness = unsafe {
        let sub_len =
            u64::from_le_bytes(core::slice::from_raw_parts(ptr, 8).try_into().unwrap()) as usize;
        ptr = ptr.add(8);
        bincode::serde::decode_from_slice(
            core::slice::from_raw_parts(ptr, sub_len),
            bincode::config::standard(),
        )
        .map(|(v, _)| v)
        .expect("Deserialization failed")
    };

    // Validate the block
    let execution_witness = witness.witness().clone();
    let block_hash = validate_block_stateless(block, execution_witness, chain_spec)
        .expect("Block validation failed");

    zisk_println!("Block validation succeeded!");
    zisk_println!(
        "Execution summary:\n  - Chain: {} (ID: {})\n  - Block Number: {}\n  - Block Hash: {}\n  - Transaction Count: {}\n  - Gas Consumed: {}",
        chain, chain_id, block_number, block_hash, tx_count, gas_used
    );

    // Commit to block hash as the output
    unsafe {
        let bytes = bincode::serde::encode_to_vec(&block_hash, bincode::config::standard())
            .expect("Serialization failed");
        write_output(bytes.as_ptr(), bytes.len());
    }

    unsafe { zkvm_deinit() };
}
