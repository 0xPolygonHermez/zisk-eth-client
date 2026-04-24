#![no_main]
ziskos::entrypoint!(main);

fn main() {
    guest_reth::run();
}
