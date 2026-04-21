#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use ethrex_common::DefaultUint256Ops;
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use ethrex_crypto::NativeCrypto;

mod impls;

#[derive(Debug)]
pub struct ZiskAccelerator {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    native_crypto: NativeCrypto,
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    native_uint256_ops: DefaultUint256Ops,
}

impl Default for ZiskAccelerator {
    fn default() -> Self {
        Self {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            native_crypto: NativeCrypto,
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            native_uint256_ops: DefaultUint256Ops,
        }
    }
}
