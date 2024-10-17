//! A simple program that takes a number `n` as input, and computes the SHA256 hash `n` times.

// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM
#![no_main]
ziskos::entrypoint!(main);

use io::WitnessInput;
use rsp_client_executor::io::ClientExecutorInput;
use rsp_client_executor::*;
use sha2::{Digest, Sha256};
use core::error;
use std::convert::TryInto;
use ziskos::{read_input, write_output};

fn main() {
    // Get the input slice from ziskos
    let input  = read_input();

    let input = bincode::deserialize::<ClientExecutorInput>(&input).unwrap();
    // Execute the block.

    let executor = ClientExecutor;
    let header = executor
        .execute::<EthereumVariant>(input)
        .expect("failed to execute client");
    let block_hash = header.hash_slow();

    println!("block_hash: {:?}", block_hash);
    
    //print!("input: {:?}", input.current_block.state_root);
    // Write the output using ziskos
    //let output = format!("n:{} {:?}\n", n, out);
    //write_output(output.as_bytes(), output.len());
}
