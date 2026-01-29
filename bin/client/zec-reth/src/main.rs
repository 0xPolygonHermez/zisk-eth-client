#![no_main]
ziskos::entrypoint!(main);

use ziskos::{read_input_slice, set_output};

mod guest;

use guest::{StatelessValidatorRethInput, validate_block};
use stateless_validator_common::new_payload_request::NewPayloadRequest;

fn main() {
    // Read and deserialize input
    let input = read_input_slice();
    let input: StatelessValidatorRethInput =
        bincode::deserialize(&input).expect("Failed to deserialize input");

    // Extract block number from the payload request
    let block_number = match &input.new_payload_request {
        NewPayloadRequest::Bellatrix(req) => req.execution_payload.block_number,
        NewPayloadRequest::Capella(req) => req.execution_payload.block_number,
        NewPayloadRequest::Deneb(req) => req.execution_payload.block_number,
        NewPayloadRequest::ElectraFulu(req) => req.execution_payload.block_number,
    };

    println!("Executing block validation for block: {}", block_number);

    // Validate the block
    let block_hash = validate_block(input).expect("Block validation failed");

    // Write block_hash value to the public output
    for (index, chunk) in block_hash.to_vec().chunks(4).enumerate() {
        let value = u32::from_le_bytes(chunk.try_into().unwrap());
        set_output(index, value);
    }

    // Print block number and calculated hash
    println!("Block validation succeeded! Block: {}. Data hash: {}", block_number, block_hash);
}
