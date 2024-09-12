/*
TAKEN from group crate as they don't expose wnaf.
Add proper reference here

*/

/// This struct represents a view of a sequence of bytes as a sequence of
/// `u64` limbs in little-endian byte order. It maintains a current index, and
/// allows access to the limb at that index and the one following it. Bytes
/// beyond the end of the original buffer are treated as zero.
struct LimbBuffer<'a> {
    buf: &'a [u8],
    cur_idx: usize,
    cur_limb: u64,
    next_limb: u64,
}

impl<'a> LimbBuffer<'a> {
    fn new(buf: &'a [u8]) -> Self {
        let mut ret = Self {
            buf,
            cur_idx: 0,
            cur_limb: 0,
            next_limb: 0,
        };

        // Initialise the limb buffers.
        ret.increment_limb();
        ret.increment_limb();
        ret.cur_idx = 0usize;

        ret
    }

    fn increment_limb(&mut self) {
        self.cur_idx += 1;
        self.cur_limb = self.next_limb;
        match self.buf.len() {
            // There are no more bytes in the buffer; zero-extend.
            0 => self.next_limb = 0,

            // There are fewer bytes in the buffer than a u64 limb; zero-extend.
            x @ 1..=7 => {
                let mut next_limb = [0; 8];
                next_limb[..x].copy_from_slice(self.buf);
                self.next_limb = u64::from_le_bytes(next_limb);
                self.buf = &[];
            }

            // There are at least eight bytes in the buffer; read the next u64 limb.
            _ => {
                let (next_limb, rest) = self.buf.split_at(8);
                self.next_limb = u64::from_le_bytes(next_limb.try_into().unwrap());
                self.buf = rest;
            }
        }
    }

    fn get(&mut self, idx: usize) -> (u64, u64) {
        assert!([self.cur_idx, self.cur_idx + 1].contains(&idx));
        if idx > self.cur_idx {
            self.increment_limb();
        }
        (self.cur_limb, self.next_limb)
    }
}

/// Replaces the contents of `wnaf` with the w-NAF representation of a little-endian
/// scalar.
pub(crate) fn wnaf_form<S: AsRef<[u8]>>(wnaf: &mut Vec<i64>, c: S, window: usize) {
    // Required by the NAF definition
    debug_assert!(window >= 2);
    // Required so that the NAF digits fit in i64
    debug_assert!(window <= 64);

    let bit_len = c.as_ref().len() * 8;

    wnaf.truncate(0);
    wnaf.reserve(bit_len);

    // Initialise the current and next limb buffers.
    let mut limbs = LimbBuffer::new(c.as_ref());

    let width = 1u64 << window;
    let window_mask = width - 1;

    let mut pos = 0;
    let mut carry = 0;
    while pos < bit_len {
        // Construct a buffer of bits of the scalar, starting at bit `pos`
        let u64_idx = pos / 64;
        let bit_idx = pos % 64;
        let (cur_u64, next_u64) = limbs.get(u64_idx);
        let bit_buf = if bit_idx + window < 64 {
            // This window's bits are contained in a single u64
            cur_u64 >> bit_idx
        } else {
            // Combine the current u64's bits with the bits from the next u64
            (cur_u64 >> bit_idx) | (next_u64 << (64 - bit_idx))
        };

        // Add the carry into the current window
        let window_val = carry + (bit_buf & window_mask);

        if window_val & 1 == 0 {
            // If the window value is even, preserve the carry and emit 0.
            // Why is the carry preserved?
            // If carry == 0 and window_val & 1 == 0, then the next carry should be 0
            // If carry == 1 and window_val & 1 == 0, then bit_buf & 1 == 1 so the next carry should be 1
            wnaf.push(0);
            pos += 1;
        } else {
            wnaf.push(if window_val < width / 2 {
                carry = 0;
                window_val as i64
            } else {
                carry = 1;
                (window_val as i64).wrapping_sub(width as i64)
            });
            wnaf.extend(std::iter::repeat(0).take(window - 1));
            pos += window;
        }
    }
}
