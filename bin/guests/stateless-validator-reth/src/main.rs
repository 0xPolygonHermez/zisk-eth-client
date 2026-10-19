#![no_main]
ziskos::entrypoint!(main);

fn main() {
    guest_reth::run();
}

// The `critical-section` crate (pulled in through the no_std stateless
// validator dependencies) needs a platform implementation. The guest is
// single-threaded, so acquire/release are no-ops.
#[unsafe(no_mangle)]
fn _critical_section_1_0_acquire() -> u64 {
    0
}

#[unsafe(no_mangle)]
fn _critical_section_1_0_release(_: u64) {}
