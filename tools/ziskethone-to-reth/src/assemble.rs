//! Assemble a `StatelessInput` from a decoded ZEG0 container plus the block
//! header fetched over RPC.
//!
//! ZEG0 omits the current block's execution outputs (`state_root`,
//! `receipts_root`, `logs_bloom`, `gas_used`, `blob_gas_used`) because the guest
//! recomputes them, and it stores the parent's *state root* where a header would
//! carry `parent_hash`. The RPC header supplies all of those; everything else —
//! transactions, withdrawals, ancestors, code, and the pre-state trie — comes
//! from the file.

use alloy_consensus::Header;
use alloy_eips::eip2718::Decodable2718;
use alloy_eips::eip4895::{Withdrawal, Withdrawals};
use alloy_primitives::Bytes;
use alloy_rlp::Encodable;
use anyhow::{ensure, Context, Result};
use reth_ethereum_primitives::{Block, BlockBody, TransactionSigned};
use stateless_reth::{ExecutionWitness, StatelessInput};

use crate::zeg0::reader::{PrevBlock, Zeg0};

/// Re-encode an ancestor header to its canonical RLP, truncated to the field
/// count the encoder recorded. Header layout grew across hardforks, so a fixed
/// field list would produce the wrong hash for pre-Pectra ancestors.
pub fn ancestor_header_rlp(p: &PrevBlock) -> Result<Bytes> {
    // The encoder only ever emits these counts (`sections::write_previous_blocks`).
    // Reject anything else rather than silently dropping trailing fields — the
    // symptom would be a wrong ancestor hash, surfacing far from the cause.
    ensure!(
        matches!(p.field_count, 15 | 16 | 17 | 20 | 21),
        "ancestor block {} has an unrecognised header field count {} (expected 15, 16, 17, 20 or 21)",
        p.number,
        p.field_count
    );

    let mut h = Header {
        parent_hash: p.parent_hash,
        ommers_hash: p.ommers_hash,
        beneficiary: p.beneficiary,
        state_root: p.state_root,
        transactions_root: p.transactions_root,
        receipts_root: p.receipts_root,
        logs_bloom: alloy_primitives::Bloom::from_slice(&p.logs_bloom),
        difficulty: p.difficulty,
        number: p.number,
        gas_limit: p.gas_limit,
        gas_used: p.gas_used,
        timestamp: p.timestamp,
        extra_data: Bytes::from(p.extra_data.clone()),
        mix_hash: p.mix_hash,
        nonce: p.nonce.into(),
        ..Default::default()
    };
    // Optional fields are gated on the recorded field count, matching the
    // encoder's own fork ladder.
    if p.field_count >= 16 {
        h.base_fee_per_gas = Some(
            p.base_fee_per_gas
                .try_into()
                .ok()
                .with_context(|| format!("ancestor {} base_fee exceeds u64", p.number))?,
        );
    }
    if p.field_count >= 17 {
        h.withdrawals_root = Some(p.withdrawals_root);
    }
    if p.field_count >= 20 {
        h.blob_gas_used = Some(p.blob_gas_used);
        h.excess_blob_gas = Some(p.excess_blob_gas);
        h.parent_beacon_block_root = Some(p.parent_beacon_block_root);
    }
    if p.field_count >= 21 {
        h.requests_hash = Some(p.requests_hash);
    }

    let mut buf = Vec::new();
    h.encode(&mut buf);
    Ok(Bytes::from(buf))
}

/// Build the block: header from RPC, body from the ZEG0 file.
pub fn build_block(zeg: &Zeg0, header: Header) -> Result<Block> {
    let mut transactions = Vec::with_capacity(zeg.transactions.len());
    for (i, raw) in zeg.transactions.iter().enumerate() {
        let tx = TransactionSigned::decode_2718(&mut raw.as_ref())
            .with_context(|| format!("decoding transaction {i} from its EIP-2718 envelope"))?;
        transactions.push(tx);
    }

    // Post-merge blocks have no ommers, which is all this tool targets. Assert it
    // rather than assume: an empty body against a header claiming ommers would be
    // an invalid block, and failing here is far clearer than a mismatch later.
    ensure!(
        header.ommers_hash == alloy_consensus::EMPTY_OMMER_ROOT_HASH,
        "block {} has ommers (ommers_hash {}), which this tool does not reconstruct",
        header.number,
        header.ommers_hash
    );

    // `withdrawals_root` on the header tells us whether the body should carry a
    // withdrawals list at all (pre-Shanghai blocks must leave it absent).
    let withdrawals = header.withdrawals_root.map(|_| {
        Withdrawals::new(
            zeg.withdrawals
                .iter()
                .map(|w| Withdrawal {
                    index: w.index,
                    validator_index: w.validator_index,
                    address: w.address,
                    amount: w.amount,
                })
                .collect(),
        )
    });

    Ok(Block {
        header,
        body: BlockBody {
            transactions,
            ommers: Vec::new(),
            withdrawals,
        },
    })
}

/// Assemble the full `StatelessInput`.
pub fn build_stateless_input(
    zeg: &Zeg0,
    header: Header,
    state: Vec<Bytes>,
    keys: Vec<Bytes>,
) -> Result<StatelessInput> {
    let block = build_block(zeg, header)?;

    let headers = zeg
        .prev_blocks
        .iter()
        .map(ancestor_header_rlp)
        .collect::<Result<Vec<_>>>()?;

    let witness = ExecutionWitness {
        state,
        codes: zeg.codes.clone(),
        keys,
        headers,
    };

    Ok(StatelessInput {
        block,
        witness,
        chain_config: reth_chainspec::mainnet_chain_config(),
    })
}
