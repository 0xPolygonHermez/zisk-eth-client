// #![no_main]
// ziskos::entrypoint!(main);

use rsp_client_executor::{executor::EthClientExecutor, io::EthClientExecutorInput};
use std::{env, fs, sync::Arc};
// use ziskos::{read_input, set_output};

fn main() {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <input_file.bin>", args[0]);
        std::process::exit(1);
    }
    
    let input_file = &args[1];
    
    // Read binary input from file
    let input: Vec<u8> = match fs::read(input_file) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_file, e);
            std::process::exit(1);
        }
    };

    let input = bincode::deserialize::<EthClientExecutorInput>(&input).unwrap();
    let block_number = input.current_block.number;

    println!("Executing {} block", block_number);

    // Execute the block.
    let executor = EthClientExecutor::eth(
        Arc::new(
            (&input.genesis)
                .try_into()
                .expect("Failed to convert genesis block into the required type"),
        ),
        input.custom_beneficiary,
    );
    let header = executor.execute(input).expect("Failed to execute client");

    // Calculate block hash
    let block_hash = header.hash_slow();

    // Write block_hash value to the public output
    // for (index, chunk) in block_hash.to_vec().chunks(4).enumerate() {
    //    let value = u32::from_le_bytes(chunk.try_into().unwrap());
    //    set_output(index, value);
    // }

    // Print block number and calculated hash
    println!("Block number: {}, hash: {}", block_number, block_hash);
}
