// Note: Any mention of field elements in this file and in general in the codebase
// refers to the scalar field.

/// The number of bytes needed to represent a field element.
///
/// Note: This is originally specified in the eip-4844 specs.
///
/// See: https://github.com/ethereum/EIPs/blob/master/EIPS/eip-4844.md
pub const BYTES_PER_FIELD_ELEMENT: usize = 32;

/// The number of field elements needed to represent a blob.
///
/// Note: This is originally specified in the eip-4844 specs.
///
/// See: https://github.com/ethereum/EIPs/blob/master/EIPS/eip-4844.md
pub const FIELD_ELEMENTS_PER_BLOB: usize = 4096;

/// The number of bytes needed to represent a blob.
pub const BYTES_PER_BLOB: usize = FIELD_ELEMENTS_PER_BLOB * BYTES_PER_FIELD_ELEMENT;

/// The number of bytes needed to represent a G1 element.
pub(crate) const BYTES_PER_G1_POINT: usize = 48;

/// The number of bytes needed to represent a commitment.
///
/// Note: commitments are G1 elements.
pub const BYTES_PER_COMMITMENT: usize = BYTES_PER_G1_POINT;
