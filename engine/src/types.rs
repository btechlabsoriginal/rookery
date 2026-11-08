use std::fmt;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Color { White, Black }
impl Color {
    #[inline] pub const fn idx(self) -> usize { match self { Self::White => 0, Self::Black => 1 } }
    #[inline] pub const fn other(self) -> Self { match self { Self::White => Self::Black, Self::Black => Self::White } }
    #[inline] pub const fn pawn_step(self) -> i8 { match self { Self::White => 8, Self::Black => -8 } }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Piece { Pawn, Knight, Bishop, Rook, Queen, King }
impl Piece {
    pub const ALL: [Self; 6] = [Self::Pawn, Self::Knight, Self::Bishop, Self::Rook, Self::Queen, Self::King];
    #[inline] pub const fn idx(self) -> usize { self as usize }
    pub const fn from_index(n: u8) -> Self {
        match n { 0 => Self::Pawn, 1 => Self::Knight, 2 => Self::Bishop, 3 => Self::Rook, 4 => Self::Queen, _ => Self::King }
    }
    pub const fn promotion_letter(self) -> char {
        match self { Self::Knight => 'n', Self::Bishop => 'b', Self::Rook => 'r', _ => 'q' }
    }
}

// Mailbox codes: 0 = empty, 1..6 = white P,N,B,R,Q,K; 7..12 = black.
#[inline] pub const fn encode(color: Color, piece: Piece) -> u8 { (color.idx() * 6 + piece.idx() + 1) as u8 }
#[inline] pub const fn decode(code: u8) -> (Color, Piece) {
    let n = code - 1;
    (if n < 6 { Color::White } else { Color::Black }, Piece::from_index(n % 6))
}

pub const CAPTURE: u8 = 1;
pub const EN_PASSANT: u8 = 2;
pub const CASTLE: u8 = 4;
pub const DOUBLE_PUSH: u8 = 8;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Move {
    pub from: u8,
    pub to: u8,
    pub promotion: Option<Piece>,
    pub flags: u8,
}
impl Move {
    #[inline] pub const fn new(from: u8, to: u8, flags: u8) -> Self { Self { from, to, flags, promotion: None } }
    #[inline] pub const fn promoted(from: u8, to: u8, piece: Piece, flags: u8) -> Self { Self { from, to, promotion: Some(piece), flags } }
    #[inline] pub const fn is_capture(self) -> bool { self.flags & CAPTURE != 0 }
    pub fn uci(self) -> String {
        let mut out = format!("{}{}", square_name(self.from), square_name(self.to));
        if let Some(p) = self.promotion { out.push(p.promotion_letter()); }
        out
    }
}
impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.uci()) }
}

pub fn square_name(sq: u8) -> String {
    format!("{}{}", (b'a' + sq % 8) as char, (b'1' + sq / 8) as char)
}
pub fn parse_square(s: &str) -> Option<u8> {
    let b = s.as_bytes();
    if b.len() != 2 || !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1]) { return None; }
    Some((b[1] - b'1') * 8 + b[0] - b'a')
}
