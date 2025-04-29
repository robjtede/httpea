use byteorder::{ByteOrder, NetworkEndian};

/// Integer strictly less than 2^62.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VarInt(u64);

impl_more::forward_display!(VarInt);

impl VarInt {
    const MIN: VarInt = VarInt(0);

    const MAX: VarInt = VarInt((2 << 62) - 1);

    pub const fn new(n: u64) -> Self {
        assert!(n <= Self::MAX.0);
        Self(n)
    }

    pub fn encode(&self, n_prefix_bits: u8) -> Vec<u8> {
        // TODO: optimize this hot garbage 2am code

        debug_assert!(n_prefix_bits > 0);
        debug_assert!(n_prefix_bits <= 8);

        // if I < 2^N - 1, encode I on N bits
        // else
        //     encode (2^N - 1) on N bits
        //     I = I - (2^N - 1)
        //     while I >= 128
        //          encode (I % 128 + 128) on 8 bits
        //          I = I / 128
        //     encode I on 8 bits

        let i = self.0;

        if i < max_n_bits(n_prefix_bits as u32) {
            let mut buf = [0; 8];

            let n_bytes = bits_to_bytes(n_prefix_bits) as usize;
            NetworkEndian::write_uint(&mut buf, i, n_bytes);

            buf[..n_bytes].to_vec()
        } else {
            let mut buf = [0; 8];

            let mut pos = bits_to_bytes(n_prefix_bits) as usize;

            let prefix_bits = u64::MAX >> (64 - n_prefix_bits);
            NetworkEndian::write_uint(&mut buf, prefix_bits, pos);

            let mut i = i - max_n_bits(n_prefix_bits);

            while i >= 128 {
                let seg = (i % 128) + 128;
                NetworkEndian::write_uint(&mut buf[pos..], seg, 1);
                i /= 128;
                pos += 1;
            }

            NetworkEndian::write_uint(&mut buf[pos..], i, 1);
            pos += 1;

            buf[..pos].to_vec()
        }
    }

    pub fn decode(buf: &[u8], n_prefix_bits: u8) -> Self {
        // decode I from the next N bits
        // if I < 2^N - 1, return I
        // else
        //     M = 0
        //     repeat
        //         B = next octet
        //         I = I + (B & 127) * 2^M
        //         M = M + 7
        //     while B & 128 == 128
        //     return I

        let acc = 0;

        let mut pos = bits_to_bytes(n_prefix_bits) as usize;
        let mut i = NetworkEndian::read_uint(buf, pos);

        if i < max_n_bits(n_prefix_bits) {
            return VarInt(i);
        }

        let mut m = 0;

        loop {
            let b = buf[pos];

            i += (b & 127) as u64 * 2_u64.pow(m);
            m += 7;

            if b & 0x80 == 0x80 {
                pos += 1;
                continue;
            } else {
                break VarInt(i);
            }
        }
    }
}

fn max_n_bits(n: impl Into<u32>) -> u64 {
    2_u64.pow(n.into()) - 1
}

fn bits_to_bytes(n: impl Into<u64>) -> u64 {
    div_ceil(n, 8_u64)
}

fn div_ceil(lhs: impl Into<u64>, rhs: impl Into<u64>) -> u64 {
    let lhs = lhs.into();
    let rhs = rhs.into();

    let d = lhs / rhs;
    let r = lhs % rhs;

    if r > 0 && rhs > 0 { d + 1 } else { d }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding() {
        assert_eq!(VarInt(10).encode(5), [0b01010]);
        assert_eq!(VarInt(1337).encode(5), [0b11111, 0b10011010, 0b00001010]);
        assert_eq!(VarInt(42).encode(8), [0b101010]);
    }

    #[test]
    fn decoding() {
        assert_eq!(VarInt::decode(&[0b01010], 5), VarInt(10));
        assert_eq!(
            VarInt::decode(&[0b11111, 0b10011010, 0b00001010], 5),
            VarInt(1337)
        );
        assert_eq!(VarInt::decode(&[0b101010], 8), VarInt(42));
    }
}
