// Note: Any mention of field elements in this file and in general in the codebase
// refers to the scalar field.

/// The number of bytes needed to represent a field element.
///
/// Note: This is originally specified in the 4844 specs.
pub const BYTES_PER_FIELD_ELEMENT: usize = 32;

/// The number of field elements in a cell.
///
/// Note: This is user defined; modifying this value will change the number of proofs produced,
/// the proof generation time and the time it takes to verify a proof.
pub const FIELD_ELEMENTS_PER_CELL: usize = 64;

/// The number of field elements needed to represent a blob.
///
/// Note: This is originally specified in the 4844 specs.
pub const FIELD_ELEMENTS_PER_BLOB: usize = 4096;

/// The number of bytes needed to represent a cell.
pub const BYTES_PER_CELL: usize = FIELD_ELEMENTS_PER_CELL * BYTES_PER_FIELD_ELEMENT;

/// The factor by which we extend a blob.
///
/// Note: This is user defined; modifying this will change the number of proofs produced,
/// proof generation time and the rate of the reed-solomon code.
pub const EXTENSION_FACTOR: usize = 2;

/// The number of field elements needed to represent an extended blob.
pub const FIELD_ELEMENTS_PER_EXT_BLOB: usize = EXTENSION_FACTOR * FIELD_ELEMENTS_PER_BLOB;

/// The number of cells in an extension blob.
///
/// Note: A cell is simply a list of `FIELD_ELEMENTS_PER_CELL` field elements.
pub const CELLS_PER_EXT_BLOB: usize = FIELD_ELEMENTS_PER_EXT_BLOB / FIELD_ELEMENTS_PER_CELL;

/// The number of bytes needed to represent a G1 element.
pub(crate) const BYTES_PER_G1_POINT: usize = 48;

/// The number of bytes needed to represent a commitment.
///
/// Note: commitments are G1 elements.
pub const BYTES_PER_COMMITMENT: usize = BYTES_PER_G1_POINT;
