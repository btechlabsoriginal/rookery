//! Magic-bitboard sliders. Occupancy masks exclude the final square of each ray;
//! the attack table includes it, so an edge blocker still stops the ray.
use std::sync::OnceLock;

mod magic_numbers;
use magic_numbers::{BISHOP_MAGICS, ROOK_MAGICS};

#[derive(Debug)]
struct MagicTable {
    mask: u64,
    magic: u64,
    shift: u32,
    attacks: Box<[u64]>,
}
impl MagicTable {
    fn new(sq: u8, bishop: bool) -> Self {
        let mask = ray_mask(sq, bishop);
        let bits = mask.count_ones();
        let magic = if bishop { BISHOP_MAGICS[sq as usize] } else { ROOK_MAGICS[sq as usize] };
        let mut attacks = vec![0; 1 << bits];
        for index in 0..(1usize << bits) {
            let blockers = subset(index, mask);
            let slot = (blockers.wrapping_mul(magic) >> (64 - bits)) as usize;
            let attack = trace(sq, blockers, bishop);
            debug_assert!(attacks[slot] == 0 || attacks[slot] == attack, "magic collision at {sq}");
            attacks[slot] = attack;
        }
        Self { mask, magic, shift: 64 - bits, attacks: attacks.into_boxed_slice() }
    }
    #[inline]
    fn get(&self, occupancy: u64) -> u64 {
        let slot = ((occupancy & self.mask).wrapping_mul(self.magic) >> self.shift) as usize;
        self.attacks[slot]
    }
}

const ORTHOGONAL: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const DIAGONAL: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
#[inline] fn on_board(f: i8, r: i8) -> bool { (0..8).contains(&f) && (0..8).contains(&r) }
fn ray_mask(sq: u8, bishop: bool) -> u64 {
    let mut mask = 0;
    for &(df, dr) in if bishop { &DIAGONAL } else { &ORTHOGONAL } {
        let (mut f, mut r) = ((sq % 8) as i8 + df, (sq / 8) as i8 + dr);
        while on_board(f, r) && on_board(f + df, r + dr) {
            mask |= 1u64 << (r * 8 + f);
            f += df; r += dr;
        }
    }
    mask
}
fn trace(sq: u8, occupancy: u64, bishop: bool) -> u64 {
    let mut attacks = 0;
    for &(df, dr) in if bishop { &DIAGONAL } else { &ORTHOGONAL } {
        let (mut f, mut r) = ((sq % 8) as i8 + df, (sq / 8) as i8 + dr);
        while on_board(f, r) {
            let bit = 1u64 << (r * 8 + f);
            attacks |= bit;
            if occupancy & bit != 0 { break; }
            f += df; r += dr;
        }
    }
    attacks
}
fn subset(mut n: usize, mut mask: u64) -> u64 {
    let mut out = 0;
    while mask != 0 {
        let bit = mask & mask.wrapping_neg();
        mask ^= bit;
        if n & 1 != 0 { out |= bit; }
        n >>= 1;
    }
    out
}

pub struct Attacks {
    rook: [MagicTable; 64],
    bishop: [MagicTable; 64],
    knight: [u64; 64],
    king: [u64; 64],
    pawn: [[u64; 64]; 2],
}
impl Attacks {
    fn new() -> Self {
        const KNIGHT: [(i8, i8); 8] = [(1,2),(2,1),(-1,2),(-2,1),(1,-2),(2,-1),(-1,-2),(-2,-1)];
        const KING: [(i8, i8); 8] = [(1,0),(-1,0),(0,1),(0,-1),(1,1),(-1,1),(1,-1),(-1,-1)];
        let mut knight = [0;64]; let mut king = [0;64]; let mut pawn = [[0;64];2];
        for sq in 0..64 {
            let f = (sq % 8) as i8; let r = (sq / 8) as i8;
            for &(df,dr) in &KNIGHT { if on_board(f+df,r+dr) { knight[sq] |= 1u64 << ((r+dr)*8+f+df); } }
            for &(df,dr) in &KING { if on_board(f+df,r+dr) { king[sq] |= 1u64 << ((r+dr)*8+f+df); } }
            for &df in &[-1i8,1] {
                if on_board(f+df,r+1) { pawn[0][sq] |= 1u64 << ((r+1)*8+f+df); }
                if on_board(f+df,r-1) { pawn[1][sq] |= 1u64 << ((r-1)*8+f+df); }
            }
        }
        Self { rook: std::array::from_fn(|sq| MagicTable::new(sq as u8, false)),
               bishop: std::array::from_fn(|sq| MagicTable::new(sq as u8, true)), knight, king, pawn }
    }
    #[inline] pub fn rook(&self, sq: u8, occ: u64) -> u64 { self.rook[sq as usize].get(occ) }
    #[inline] pub fn bishop(&self, sq: u8, occ: u64) -> u64 { self.bishop[sq as usize].get(occ) }
    #[inline] pub fn queen(&self, sq: u8, occ: u64) -> u64 { self.rook(sq,occ) | self.bishop(sq,occ) }
    #[inline] pub fn knight(&self, sq: u8) -> u64 { self.knight[sq as usize] }
    #[inline] pub fn king(&self, sq: u8) -> u64 { self.king[sq as usize] }
    #[inline] pub fn pawn(&self, color: usize, sq: u8) -> u64 { self.pawn[color][sq as usize] }
}
static ATTACKS: OnceLock<Attacks> = OnceLock::new();
#[inline] pub fn tables() -> &'static Attacks { ATTACKS.get_or_init(Attacks::new) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn magic_tables_match_ray_tracing() {
        let t = tables();
        for sq in 0u8..64 {
            for bishop in [true,false] {
                let mask = ray_mask(sq,bishop);
                for i in 0..(1usize<<mask.count_ones()) {
                    let occ = subset(i,mask);
                    let found = if bishop { t.bishop(sq,occ) } else { t.rook(sq,occ) };
                    assert_eq!(found,trace(sq,occ,bishop),"square={sq}, subset={i}");
                }
            }
        }
    }
}
