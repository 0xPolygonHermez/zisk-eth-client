use std::sync::Arc;

use alloy_consensus::crypto::install_default_provider;
use revm::install_crypto;

use super::{
    CustomEvmCrypto, extract_block_info, get_chain_name, get_chain_spec, validate_block_stateless,
    verify_signatures, RethInputPublic, RethInputWitness,
};

/// One-time crypto provider setup. Must be called before `run()` on native builds.
/// Safe to call multiple times; subsequent calls are no-ops.
pub fn init_crypto() {
    install_crypto(CustomEvmCrypto::default());
    let _ = install_default_provider(Arc::new(CustomEvmCrypto::default()));
}

/// Run the full block validation: read inputs, validate, commit output.
pub fn run() {
    init_crypto();
    let public: RethInputPublic = ziskos::io::read();

    let chain_config = public.chain_config().clone();
    let block = public.block().clone();
    let (block_number, gas_used, tx_count) = extract_block_info(&block);
    let chain_id = chain_config.chain_id;
    let chain = get_chain_name(chain_id);
    println!(
        "Executing block validation for {} Block #{} ({} txs)",
        chain, block_number, tx_count
    );

    let chain_spec = get_chain_spec(&chain_config);
    let block = verify_signatures(block, chain_spec.clone(), public.public_keys)
        .expect("Signature verification failed");

    let witness: RethInputWitness = ziskos::io::read();

    let execution_witness = witness.witness().clone();
    let block_hash = validate_block_stateless(block, execution_witness, chain_spec)
        .expect("Block validation failed");

    ziskos::io::commit(&block_hash);

    println!("Block validation succeeded!");
    println!(
        "Execution summary:\n  - Chain: {} (ID: {})\n  - Block Number: {}\n  - Block Hash: {}\n  - Transaction Count: {}\n  - Gas Consumed: {}",
        chain, chain_id, block_number, block_hash, tx_count, gas_used
    );
}
