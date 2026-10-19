use std::sync::{Arc, Once};

use alloy_consensus::crypto::install_default_provider;
use revm::install_crypto;

use guest_common::chain::chain_name;

use super::{validate, CustomEvmCrypto, InputPeek, SszDecode, StatelessValidationResult};

/// Install the ZisK-accelerated crypto backends for revm and alloy. Idempotent.
pub fn install_zisk_crypto() {
    static INSTALL_CRYPTO: Once = Once::new();
    INSTALL_CRYPTO.call_once(|| {
        install_crypto(CustomEvmCrypto);
        install_default_provider(Arc::new(CustomEvmCrypto))
            .expect("Failed to install default crypto provider");
    });
}

/// Run the full block validation: read the stateless input, validate, commit
/// the output. Printing costs cycles, so output is kept minimal.
pub fn run() {
    install_zisk_crypto();

    // The whole input is one frame: `schema_id || SSZ(StatelessInput)`.
    let input = ziskos::io::read_slice();

    // Block facts come from fixed offsets of the input, not from decoding it:
    // the validator decodes internally and takes only bytes, and a second
    // full SSZ decode would cost cycles for no validation value.
    match InputPeek::peek(&input) {
        Some(block) => println!(
            "Executing block validation for {} Block #{} ({} txs)",
            chain_name(block.chain_id),
            block.block_number,
            block.tx_count,
        ),
        None => println!("Executing block validation"),
    }

    // Validate the block
    let output = validate(&input);

    // Commit the validation result as the public output
    ziskos::io::commit_slice(&output);

    let result = StatelessValidationResult::from_ssz_bytes(&output)
        .expect("validator output is always a well-formed StatelessValidationResult");
    if result.successful_validation {
        println!("Block validation succeeded!");
    } else {
        println!("Block validation FAILED!");
    }
}
