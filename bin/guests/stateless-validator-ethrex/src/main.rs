#![no_main]
ziskos::entrypoint!(main);

fn main() {
    guest_ethrex::run();
}
