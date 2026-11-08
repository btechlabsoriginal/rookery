//! Fixed-seed SplitMix64: positions have reproducible keys across native and WASM.
use std::sync::OnceLock;

pub struct Zobrist {
    pub piece: [[[u64; 64]; 6]; 2],
    pub castle: [u64; 16],
    pub en_passant: [u64; 8],
    pub black_to_move: u64,
}
impl Zobrist {
    fn new() -> Self {
        let mut state = 0x52304F4F4B455259u64;
        fn next(s: &mut u64) -> u64 {
            *s = s.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = *s;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        }
        let piece = std::array::from_fn(|_| std::array::from_fn(|_| std::array::from_fn(|_| next(&mut state))));
        let castle = std::array::from_fn(|_| next(&mut state));
        let en_passant = std::array::from_fn(|_| next(&mut state));
        let black_to_move = next(&mut state);
        Self { piece, castle, en_passant, black_to_move }
    }
}
static KEYS: OnceLock<Zobrist> = OnceLock::new();
#[inline] pub fn keys() -> &'static Zobrist { KEYS.get_or_init(Zobrist::new) }
