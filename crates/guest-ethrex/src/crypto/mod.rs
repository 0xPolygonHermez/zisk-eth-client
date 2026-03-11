#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use ethrex_crypto::NativeCrypto;

mod impls;

#[derive(Debug)]
pub struct ZiskCrypto {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    native_crypto: NativeCrypto,
}

impl Default for ZiskCrypto {
    fn default() -> Self {
        Self {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            native_crypto: NativeCrypto,
        }
    }
}
