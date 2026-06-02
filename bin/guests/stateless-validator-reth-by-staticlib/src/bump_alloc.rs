use core::alloc::{GlobalAlloc, Layout};

// critical-section implementation for single-threaded bare-metal: no-op.
// Required by once_cell (pulled in via reth-evm-ethereum) on targets without OS support.
#[unsafe(no_mangle)]
unsafe extern "C" fn _critical_section_1_0_acquire() -> u8 {
    0
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _critical_section_1_0_release(_token: u8) {}

unsafe extern "C" {
    static _kernel_heap_bottom: u8;
    static _kernel_heap_top: u8;
}

static mut HEAP_POS: usize = 0;
static mut HEAP_TOP: usize = 0;

/// Initialize the heap from linker-script symbols. Must be called before any allocation.
pub unsafe fn init_heap() {
    unsafe {
        HEAP_POS = &raw const _kernel_heap_bottom as *const u8 as usize;
        HEAP_TOP = &raw const _kernel_heap_top as *const u8 as usize;
    }
}

#[inline(always)]
unsafe fn bump_alloc(bytes: usize, align: usize) -> *mut u8 {
    // SAFETY: Single-threaded bare-metal; nothing else touches these while we work.
    let mut heap_pos = unsafe { HEAP_POS };

    debug_assert!(align.is_power_of_two(), "align must be a power of two");

    // checked_add prevents wrap-around from silently corrupting heap_pos.
    let offset = heap_pos & (align - 1);
    if offset != 0 {
        heap_pos = heap_pos.checked_add(align - offset).expect("heap_pos alignment overflow");
    }

    let ptr = heap_pos as *mut u8;

    // Guard against integer overflow in the size addition *before* the OOM check.
    // Without this, a huge `bytes` wraps heap_pos to a small number, the OOM check
    // passes on the wrapped value, and HEAP_POS is corrupted.
    heap_pos = heap_pos.checked_add(bytes).expect("allocation size overflow");

    if unsafe { HEAP_TOP } < heap_pos {
        panic!("OOM: heap exhausted");
    }

    unsafe { HEAP_POS = heap_pos };

    ptr
}

pub struct BumpPointerAlloc;

unsafe impl GlobalAlloc for BumpPointerAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { bump_alloc(layout.size(), layout.align()) }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}

    // The bare-metal heap is pre-zeroed, so allocating is sufficient.
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { bump_alloc(layout.size(), layout.align()) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { bump_alloc(new_size, layout.align()) };
        unsafe { core::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size)) };
        new_ptr
    }
}

#[global_allocator]
static ALLOC: BumpPointerAlloc = BumpPointerAlloc;
