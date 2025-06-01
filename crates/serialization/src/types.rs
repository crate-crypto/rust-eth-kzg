use crate::constants::{
    BYTES_PER_BLOB, BYTES_PER_CELL, BYTES_PER_COMMITMENT, BYTES_PER_FIELD_ELEMENT,
};

/// `BlobRef` denotes a references to an opaque Blob.
///
/// Note: This library never returns a Blob, which is why we
/// do not have a Blob type.
pub type BlobRef<'a> = &'a [u8; BYTES_PER_BLOB];

/// `Bytes48Ref` denotes a reference to an untrusted cryptographic type
/// that can be represented in 48 bytes. This will be either a
/// purported `KZGProof` or a purported `KZGCommitment`.
pub type Bytes48Ref<'a> = &'a [u8; 48];

/// Cell contains a group of evaluations on a coset that one would like to
/// make and verify opening proofs about.
///
/// Note: These are heap allocated.
pub type Cell = Box<[u8; BYTES_PER_CELL]>;

/// `CellRef` contains a reference to a Cell.
///
/// Note: Similar to Blob, the library takes in references
/// to Cell and returns heap allocated instances as return types.
pub type CellRef<'a> = &'a [u8; BYTES_PER_CELL];

/// `KZGProof` denotes a 48 byte commitment to a polynomial
/// that one can use to either:
///     - Prove that a polynomial f(x) was correctly evaluated on a coset `H` and returned a set of points (7594)
///     - Prove that a polynomial f(x) was correctly evaluated at some random point (4844)
///
/// Note: This is reusing the same type for two different proofs.
pub type KZGProof = [u8; BYTES_PER_COMMITMENT];

/// `KZGCommitment` denotes a 48 byte commitment to a polynomial f(x)
/// that we would like to make and verify opening proofs about.
pub type KZGCommitment = [u8; BYTES_PER_COMMITMENT];

/// `SerializedScalar` denotes a 32 byte field element.
pub type SerializedScalar = [u8; BYTES_PER_FIELD_ELEMENT];
