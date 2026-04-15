#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use revm::precompile::DefaultCrypto;
#[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
use revm::primitives::DefaultUint256Ops;

mod impls;

#[derive(Debug)]
pub struct CustomEvmCrypto {
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    default_crypto: DefaultCrypto,
    #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
    native_uint256_ops: DefaultUint256Ops,
}

impl Default for CustomEvmCrypto {
    fn default() -> Self {
        Self {
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            default_crypto: DefaultCrypto,
            #[cfg(not(all(target_os = "zkvm", target_vendor = "zisk")))]
            native_uint256_ops: DefaultUint256Ops,
        }
    }
}
