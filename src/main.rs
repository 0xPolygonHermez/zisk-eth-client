#![no_main]
ziskos::entrypoint!(main);

use rsp_client_executor::io::ClientExecutorInput;
use rsp_client_executor::*;
use ziskos::read_input;

fn main() {
    // Get the input slice from ziskos
    let input  = read_input();

    let input = bincode::deserialize::<ClientExecutorInput>(&input).unwrap();
    let block_number = input.current_block.number;

    println!("executing block {}", block_number);

    // Execute the block.
    let executor = ClientExecutor;
    let header = executor
        .execute::<EthereumVariant>(input)
        .expect("failed to execute client");

    // Calculate block hash    
    let block_hash = header.hash_slow();
      
    // Print block number and calculated hash  
    println!("block number: {}, hash: {}\n", block_number, block_hash);
}
