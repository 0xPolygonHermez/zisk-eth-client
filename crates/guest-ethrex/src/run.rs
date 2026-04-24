use super::{EthrexInput, ZiskCrypto, extract_block_info, get_chain_name, validate_block};

pub fn run() {
    let input: EthrexInput = EthrexInput::deserialize(&ziskos::io::read_input_slice())
        .expect("Failed to deserialize EthrexInput");

    let chain_config = input.witness().chain_config;
    let block = input.block();
    let (block_number, gas_used, tx_count) = extract_block_info(block);
    let chain_id = chain_config.chain_id;
    let chain = get_chain_name(chain_id);
    println!(
        "Executing block validation for {} Block #{} ({} txs)",
        chain, block_number, tx_count
    );

    let crypto = ZiskCrypto::default();
    let block_hash =
        validate_block(input, std::sync::Arc::new(crypto)).expect("Block validation failed");

    ziskos::io::commit(&block_hash);

    println!("Block validation succeeded!");
    println!(
        "Execution summary:\n  - Chain: {} (ID: {})\n  - Block Number: {}\n  - Block Hash: {:?}\n  - Transaction Count: {}\n  - Gas Consumed: {}",
        chain, chain_id, block_number, block_hash, tx_count, gas_used
    );
}
