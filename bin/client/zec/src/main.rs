#![no_main]
ziskos::entrypoint!(main);

use ziskos::{read_input_slice, set_output};

mod guest;

use guest::{StatelessValidatorRethInput, validate_block};

fn main() {
    let input = read_input_slice();

    let input: StatelessValidatorRethInput =
        bincode::deserialize(&input).expect("Failed to deserialize input");

    println!("Executing block validation");

    let output = validate_block(input);

    // Write new_payload_request_root to the public output
    for (index, chunk) in output.new_payload_request_root.chunks(4).enumerate() {
        let value = u32::from_le_bytes(chunk.try_into().unwrap());
        set_output(index, value);
    }

    if output.successful_block_validation {
        println!("Block validation succeeded");
    } else {
        println!("Block validation failed");
    }
}
