//! Bit-level primitives shared by the block decoders.

/// Unpack little-endian bit-packed signed integers of `bits` width (1..=32).
pub fn packed_values(data: &[u8], bits: usize) -> PackedValues<'_> {
    debug_assert!((1..=32).contains(&bits));
    PackedValues {
        data: data.iter(),
        bits,
        mask: (1u64 << bits) - 1,
        accumulator: 0,
        available: 0,
    }
}

pub struct PackedValues<'a> {
    data: std::slice::Iter<'a, u8>,
    bits: usize,
    mask: u64,
    accumulator: u64,
    available: usize,
}

impl Iterator for PackedValues<'_> {
    type Item = i32;

    fn next(&mut self) -> Option<i32> {
        while self.available < self.bits {
            let &byte = self.data.next()?;
            self.accumulator |= (byte as u64) << self.available;
            self.available += 8;
        }
        let value = sign_extend((self.accumulator & self.mask) as u32, self.bits);
        self.accumulator >>= self.bits;
        self.available -= self.bits;
        Some(value)
    }
}

/// Sign-extend the low `bits` bits of `raw` to an i32.
pub fn sign_extend(raw: u32, bits: usize) -> i32 {
    let shift = 32 - bits as u32;
    ((raw << shift) as i32) >> shift
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unpack(data: &[u8], bits: usize) -> Vec<i32> {
        packed_values(data, bits).collect()
    }

    #[test]
    fn packed_values_sign_extend() {
        // Two 4-bit values: 0x7 and 0xF (-1), then 0x8 (-8) and 0x0.
        assert_eq!(unpack(&[0xF7, 0x08], 4), vec![7, -1, -8, 0]);
    }

    #[test]
    fn packed_values_full_width() {
        assert_eq!(unpack(&(-24i32).to_le_bytes(), 32), vec![-24]);
        assert_eq!(unpack(&(-24i16).to_le_bytes(), 16), vec![-24]);
    }

    #[test]
    fn packed_values_unaligned_width() {
        // 5-bit values 1, 2, 3 packed LSB-first: 0b00011_00010_00001 = 0x0C41
        assert_eq!(unpack(&[0x41, 0x0C], 5), vec![1, 2, 3]);
    }

    #[test]
    fn packed_values_ignore_trailing_partial_value() {
        // 3 bits over one byte: two full values, two leftover bits dropped.
        assert_eq!(unpack(&[0b11_010_001], 3), vec![1, 2]);
    }
}
