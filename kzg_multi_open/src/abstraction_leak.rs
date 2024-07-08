//
// TODO: Possibly put this in the readme.
//
// This API has an abstraction leak.
//
// We explain it here, so that future maintainers are not wondering about a seamlessly unneeded property.
//
//
// Bit Reversal
//
// FK20 in and of itself does not directly require bit reversal.
//
// However, due to the input points needing to be over cosets, the evaluations/output points
// are not
//
// Now note that Fk20 will take in the original data and produce proofs over that data,
// or more correctly chunks of the data.
//
// As noted above the original data becomes scattered just due to use not having complete control
// over the input points.
//
// We can however use bit-reversal techniques to ensure that the original data is preserved.
//
// We take in the original data and then bit-reverse it. Fk20 runs over this bit-reversed data.
// We then bit-reverse the cosets, which ensures that the first N output points, will present
// the original data.
//
// Another benefit of this bit reversed way of doing things, is that you can make statements
// about chunks of the original data. ie you can easily say something about the first 32 field elements
// and you only need a single proof for that, along with the corresponding evaluations.
//
// With the "normal order", you need all of the cosets to make a statement about the first 32 field
// elements of the original data. You can see this by noting that you can read off the original data by reading column-wise
