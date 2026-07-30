//! Rebuild the revealed pre-state MPT from the ZEG0 trie-hint opcode stream.
//!
//! The stream is a depth-first transcription of the trie the witness revealed.
//! Unrevealed subtrees appear as an explicit `Op::Hash` — which is exactly what
//! an execution witness omits — so replaying the stream reproduces the same node
//! set reth's witness carried.
//!
//! Two shape transforms have to be undone:
//!   * The encoder expands an extension into a chain of single-child branches
//!     (`state_root::emit_extension`), so single-child branches fold back into
//!     extensions here. A branch with one child and no value cannot exist in a
//!     canonical MPT, so the fold is unambiguous.
//!   * Leaves carry decoded fields rather than RLP, so account and storage
//!     values are re-encoded.
//!
//! Correctness is not assumed: [`rebuild`] returns the recomputed root, and the
//! caller checks it against the parent `state_root` the container carries.

use alloy_primitives::{Bytes, B256, U256};
use anyhow::{bail, ensure, Result};
use tiny_keccak::{Hasher, Keccak};

use super::reader::Cursor;

const OP_EMPTY: u64 = 0;
const OP_HASH: u64 = 1;
const OP_EXTENSION_HASH: u64 = 2;
const OP_LEAF: u64 = 3;
const OP_BRANCH: u64 = 4;
const OP_PHANTOM_LEAF: u64 = 5;

/// keccak256(rlp("")) — the empty-trie root.
const EMPTY_TRIE_ROOT: [u8; 32] = [
    0x56, 0xe8, 0x1f, 0x17, 0x1b, 0xcc, 0x55, 0xa6, 0xff, 0x83, 0x45, 0xe6, 0x92, 0xc0, 0xf8, 0x6e,
    0x5b, 0x48, 0xe0, 0x1b, 0x99, 0x6c, 0xad, 0xc0, 0x01, 0x62, 0x2f, 0xb5, 0xe3, 0x63, 0xb4, 0x21,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum TreeKind {
    State,
    Storage,
}

/// A parsed node, before RLP encoding. Keeping the structure (rather than
/// encoding eagerly) is what makes the single-child-branch fold possible.
enum Node {
    Empty,
    /// An unrevealed subtree, known only by hash.
    Hash([u8; 32]),
    Leaf {
        path: Vec<u8>,
        /// The already-RLP-encoded value (account or storage slot).
        value: Vec<u8>,
    },
    Extension {
        path: Vec<u8>,
        child: Box<Node>,
    },
    Branch(Vec<Node>),
}

/// Result of replaying the stream.
pub struct Rebuilt {
    /// Every revealed node's RLP, in depth-first order.
    pub state: Vec<Bytes>,
    /// Plaintext preimages (20-byte addresses, 32-byte slot positions).
    pub keys: Vec<Bytes>,
    /// The recomputed state root — must equal the container's parent state root.
    pub root: B256,
}

pub fn rebuild(stream: &[u8]) -> Result<Rebuilt> {
    let mut c = Cursor::new(stream);
    let mut out = Rebuilt {
        state: Vec::new(),
        keys: Vec::new(),
        root: B256::ZERO,
    };
    let node = parse_node(&mut c, TreeKind::State, &mut out)?;
    let (rlp, reference) = encode(&node, &mut out.state);

    // A witness is a set of nodes keyed by hash; the same node RLP can be
    // reached more than once during the walk, so drop repeats rather than
    // shipping redundant bytes.
    let mut seen = std::collections::HashSet::new();
    out.state.retain(|n| seen.insert(n.clone()));
    let mut seen_keys = std::collections::HashSet::new();
    out.keys.retain(|k| seen_keys.insert(k.clone()));

    out.root = match reference {
        // A root under 32 bytes is still hashed to form the root commitment.
        NodeRef::Inline => B256::from(keccak(&rlp)),
        NodeRef::Hash(h) => B256::from(h),
        NodeRef::Empty => B256::from(EMPTY_TRIE_ROOT),
    };
    Ok(out)
}

fn parse_node(c: &mut Cursor<'_>, kind: TreeKind, out: &mut Rebuilt) -> Result<Node> {
    let op = c.u64_le()?;
    match op {
        OP_EMPTY => Ok(Node::Empty),
        OP_HASH => {
            let h: [u8; 32] = c.take(32)?.try_into().unwrap();
            Ok(Node::Hash(h))
        }
        OP_EXTENSION_HASH => {
            let path = read_nibbles(c)?;
            let h: [u8; 32] = c.take(32)?.try_into().unwrap();
            Ok(Node::Extension {
                path,
                child: Box::new(Node::Hash(h)),
            })
        }
        OP_LEAF => parse_leaf(c, kind, out),
        OP_BRANCH => {
            let mut children = Vec::with_capacity(16);
            for _ in 0..16 {
                children.push(parse_node(c, kind, out)?);
            }
            Ok(fold(Node::Branch(children)))
        }
        OP_PHANTOM_LEAF => {
            // A keyless sibling: no table row, but it still contributes its
            // hash, and its value arrives pre-encoded.
            let path = read_nibbles(c)?;
            let value = c.len_prefixed()?;
            Ok(Node::Leaf { path, value })
        }
        other => bail!("unknown ZEG0 trie opcode {other}"),
    }
}

fn parse_leaf(c: &mut Cursor<'_>, kind: TreeKind, out: &mut Rebuilt) -> Result<Node> {
    let path = read_nibbles(c)?;
    match kind {
        TreeKind::State => {
            let address = c.address()?;
            c.skip(4)?; // pad to 8-byte alignment
            let balance = c.u256_be()?;
            let nonce = c.u64_le()?;
            let code_hash = c.b256()?;

            // The account's storage subtree follows inline as a recursive node.
            let storage = parse_node(c, TreeKind::Storage, out)?;
            let (storage_rlp, storage_ref) = encode(&storage, &mut out.state);
            let storage_root = match storage_ref {
                NodeRef::Empty => EMPTY_TRIE_ROOT,
                NodeRef::Hash(h) => h,
                NodeRef::Inline => keccak(&storage_rlp),
            };

            out.keys.push(Bytes::from(address.to_vec()));

            let account = rlp_list(&[
                rlp_uint(U256::from(nonce)),
                rlp_uint(balance),
                rlp_bytes(&storage_root),
                rlp_bytes(code_hash.as_slice()),
            ]);
            Ok(Node::Leaf {
                path,
                value: account,
            })
        }
        TreeKind::Storage => {
            let position = c.b256()?;
            let value = c.b256()?;
            out.keys.push(Bytes::from(position.to_vec()));
            Ok(Node::Leaf {
                path,
                value: rlp_uint(U256::from_be_bytes(value.0)),
            })
        }
    }
}

/// `u64 count` followed by one nibble per `u64` word (low 4 bits used).
fn read_nibbles(c: &mut Cursor<'_>) -> Result<Vec<u8>> {
    let n = c.u64_le()? as usize;
    let mut v = Vec::with_capacity(n);
    for _ in 0..n {
        v.push((c.u64_le()? & 0x0f) as u8);
    }
    Ok(v)
}

/// Collapse a branch that has exactly one non-empty child and no value: in a
/// canonical MPT that node is an extension. This inverts the encoder's
/// extension-to-branch-chain expansion, merging with the child's own path so a
/// multi-nibble extension folds back in one piece.
fn fold(node: Node) -> Node {
    let Node::Branch(mut children) = node else {
        return node;
    };
    let non_empty: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, c)| !matches!(c, Node::Empty))
        .map(|(i, _)| i)
        .collect();
    if non_empty.len() != 1 {
        return Node::Branch(children);
    }
    let idx = non_empty[0];
    let child = std::mem::replace(&mut children[idx], Node::Empty);
    let mut path = vec![idx as u8];
    match child {
        Node::Extension {
            path: cpath,
            child: cchild,
        } => {
            path.extend_from_slice(&cpath);
            Node::Extension {
                path,
                child: cchild,
            }
        }
        Node::Leaf { path: cpath, value } => {
            path.extend_from_slice(&cpath);
            Node::Leaf { path, value }
        }
        other => Node::Extension {
            path,
            child: Box::new(other),
        },
    }
}

/// How a node is referenced from its parent: inline when its RLP is under 32
/// bytes, otherwise by hash.
enum NodeRef {
    Empty,
    Hash([u8; 32]),
    /// RLP under 32 bytes: the parent embeds it directly.
    Inline,
}

/// Encode `node` bottom-up, pushing every revealed node whose RLP is 32 bytes
/// or longer onto `nodes` (those are exactly the witness entries; shorter nodes
/// live inline inside their parent and are never separate entries).
fn encode(node: &Node, nodes: &mut Vec<Bytes>) -> (Vec<u8>, NodeRef) {
    match node {
        Node::Empty => (Vec::new(), NodeRef::Empty),
        Node::Hash(h) => (Vec::new(), NodeRef::Hash(*h)),
        Node::Leaf { path, value } => {
            let rlp = rlp_list(&[rlp_bytes(&hex_prefix(path, true)), rlp_bytes(value)]);
            let r = finalize(&rlp, nodes);
            (rlp, r)
        }
        Node::Extension { path, child } => {
            let (crlp, cref) = encode(child, nodes);
            let child_item = match cref {
                NodeRef::Empty => rlp_bytes(&[]),
                NodeRef::Hash(h) => rlp_bytes(&h),
                NodeRef::Inline => crlp,
            };
            let rlp = rlp_list(&[rlp_bytes(&hex_prefix(path, false)), child_item]);
            let r = finalize(&rlp, nodes);
            (rlp, r)
        }
        Node::Branch(children) => {
            let mut items = Vec::with_capacity(17);
            for ch in children {
                let (crlp, cref) = encode(ch, nodes);
                items.push(match cref {
                    NodeRef::Empty => rlp_bytes(&[]),
                    NodeRef::Hash(h) => rlp_bytes(&h),
                    NodeRef::Inline => crlp,
                });
            }
            // Item 16, the branch value, is always empty for state and storage
            // tries (the encoder drops it for the same reason).
            items.push(rlp_bytes(&[]));
            let rlp = rlp_list(&items);
            let r = finalize(&rlp, nodes);
            (rlp, r)
        }
    }
}

fn finalize(rlp: &[u8], nodes: &mut Vec<Bytes>) -> NodeRef {
    if rlp.len() >= 32 {
        nodes.push(Bytes::from(rlp.to_vec()));
        NodeRef::Hash(keccak(rlp))
    } else {
        NodeRef::Inline
    }
}

/// Hex-prefix (compact) encoding of a nibble path, per the Yellow Paper.
fn hex_prefix(nibbles: &[u8], is_leaf: bool) -> Vec<u8> {
    let mut flag = if is_leaf { 2u8 } else { 0u8 };
    let odd = nibbles.len() % 2 == 1;
    if odd {
        flag += 1;
    }
    let mut out = Vec::with_capacity(nibbles.len() / 2 + 1);
    if odd {
        out.push((flag << 4) | nibbles[0]);
        for pair in nibbles[1..].chunks(2) {
            out.push((pair[0] << 4) | pair[1]);
        }
    } else {
        out.push(flag << 4);
        for pair in nibbles.chunks(2) {
            out.push((pair[0] << 4) | pair[1]);
        }
    }
    out
}

// ----- minimal RLP -----------------------------------------------------------

fn rlp_bytes(b: &[u8]) -> Vec<u8> {
    if b.len() == 1 && b[0] < 0x80 {
        return vec![b[0]];
    }
    let mut out = encode_len(b.len(), 0x80);
    out.extend_from_slice(b);
    out
}

fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.concat();
    let mut out = encode_len(payload.len(), 0xc0);
    out.extend_from_slice(&payload);
    out
}

/// RLP of an integer: big-endian, minimal (leading zeros stripped; zero is "").
fn rlp_uint(v: U256) -> Vec<u8> {
    let be = v.to_be_bytes::<32>();
    let first = be.iter().position(|b| *b != 0).unwrap_or(be.len());
    rlp_bytes(&be[first..])
}

fn encode_len(len: usize, offset: u8) -> Vec<u8> {
    if len <= 55 {
        return vec![offset + len as u8];
    }
    let be = len.to_be_bytes();
    let first = be.iter().position(|b| *b != 0).unwrap();
    let len_bytes = &be[first..];
    let mut out = vec![offset + 55 + len_bytes.len() as u8];
    out.extend_from_slice(len_bytes);
    out
}

fn keccak(data: &[u8]) -> [u8; 32] {
    let mut h = Keccak::v256();
    h.update(data);
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

/// Guard used by the caller: the rebuilt root must match the anchor the
/// container carries, otherwise the reconstruction is wrong and the output
/// would be silently useless.
pub fn check_root(rebuilt: &Rebuilt, expected: B256) -> Result<()> {
    ensure!(
        rebuilt.root == expected,
        "rebuilt state root {} does not match the parent state root {} carried by the input \
         — the trie reconstruction is wrong, refusing to emit a bad reth input",
        rebuilt.root,
        expected
    );
    Ok(())
}
