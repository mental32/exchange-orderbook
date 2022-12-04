#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
#[repr(transparent)]
pub struct OrderIndex(pub(crate) u64);

impl OrderIndex {
    #[inline]
    pub(crate) const fn new(linear_index: u32, gen: u16) -> Self {
        let mut data = [
            0, // [0] ZERO
            0, // [1] ZERO
            0, // [2] liner index (little-endian)
            0, // [3] |
            0, // [4] |
            0, // [5] |
            0, // [6] generation counter (little-endian)
            0, // [7] |
        ];

        {
            let [a, b, c, d] = u32::to_le_bytes(linear_index);
            data[2] = a;
            data[3] = b;
            data[4] = c;
            data[5] = d;
        }

        {
            let [a, b] = u16::to_le_bytes(gen);
            data[6] = a;
            data[7] = b;
        }

        Self(u64::from_le_bytes(data))
    }

    #[inline]
    const fn to_bytes(&self) -> [u8; 8] {
        u64::to_le_bytes(self.0)
    }

    #[inline]
    pub(crate) const fn index(&self) -> u32 {
        let [_, _, a, b, c, d, _, _] = self.to_bytes();
        u32::from_le_bytes([a, b, c, d])
    }

    #[inline]
    pub(crate) const fn inc_gen(&self) -> OrderIndex {
        let [.., g1, g2] = self.to_bytes();
        let gen = u16::from_le_bytes([g1, g2]).saturating_add(1);
        Self::new(self.index(), gen)
    }
}
