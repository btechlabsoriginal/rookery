//! Rookery: a bitboard chess engine suitable for a native UCI process or WASM.
pub mod attacks;
pub mod board;
pub mod eval;
pub mod movegen;
pub mod search;
pub mod types;
pub mod zobrist;

pub use board::{Board,STARTPOS};
pub use movegen::{find_uci_move,legal_moves,perft};
pub use search::{Analysis,Limits,Searcher};
pub use types::{Color,Move,Piece};
