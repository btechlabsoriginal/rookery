use crate::{attacks::tables, board::{Board,WK,WQ,BK,BQ}, types::*};

#[inline] fn pop(mut bb:u64, mut f:impl FnMut(u8)) {
    while bb!=0 {let sq=bb.trailing_zeros() as u8;bb &= bb-1;f(sq);}
}
const PROMOTIONS: [Piece;4]=[Piece::Queen,Piece::Rook,Piece::Bishop,Piece::Knight];

/// Generate pseudo-legal moves; legality is verified by make/check/unmake.
/// Castling is special: king transit squares must already be unattacked.
pub fn pseudo_moves(b:&Board)->Vec<Move> {
    let side=b.stm;let own=b.occupancy[side.idx()];let enemy=b.occupancy[side.other().idx()];
    let occupied=b.occupancy[2];let t=tables();let mut moves=Vec::with_capacity(64);
    let pawns=b.pieces[side.idx()][Piece::Pawn.idx()];
    pop(pawns,|from| {
        let f=from%8;let rank=from/8;
        let next=(from as i16+side.pawn_step() as i16) as i16;
        let promotion_rank=if side==Color::White {6} else {1};
        let start_rank=if side==Color::White {1} else {6};
        if (0..64).contains(&next) {
            let to=next as u8;
            if b.squares[to as usize]==0 {
                if rank==promotion_rank {
                    for kind in PROMOTIONS {moves.push(Move::promoted(from,to,kind,0));}
                } else {
                    moves.push(Move::new(from,to,0));
                    if rank==start_rank {
                        let two=(next+side.pawn_step() as i16) as u8;
                        if b.squares[two as usize]==0 {moves.push(Move::new(from,two,DOUBLE_PUSH));}
                    }
                }
            }
        }
        for df in [-1i8,1] {
            if !(0..8).contains(&(f as i8+df)) {continue;}
            let dest=(from as i16+side.pawn_step() as i16+df as i16) as i16;
            if !(0..64).contains(&dest){continue;}
            let to=dest as u8;let bit=1u64<<to;
            if enemy&bit!=0 {
                // We never generate king captures; the king is attacked instead.
                if b.squares[to as usize]==encode(side.other(),Piece::King){continue;}
                if rank==promotion_rank {
                    for kind in PROMOTIONS {moves.push(Move::promoted(from,to,kind,CAPTURE));}
                } else {moves.push(Move::new(from,to,CAPTURE));}
            } else if b.ep==Some(to) {
                let victim=(to as i16-side.pawn_step() as i16) as u8;
                if b.squares[victim as usize]==encode(side.other(),Piece::Pawn) {
                    moves.push(Move::new(from,to,CAPTURE|EN_PASSANT));
                }
            }
        }
    });

    for piece in [Piece::Knight,Piece::Bishop,Piece::Rook,Piece::Queen,Piece::King] {
        pop(b.pieces[side.idx()][piece.idx()], |from| {
            let attacks=match piece {
                Piece::Knight=>t.knight(from),Piece::Bishop=>t.bishop(from,occupied),
                Piece::Rook=>t.rook(from,occupied),Piece::Queen=>t.queen(from,occupied),
                Piece::King=>t.king(from),_=>unreachable!(),
            } & !own;
            pop(attacks,|to| {
                if b.squares[to as usize]==encode(side.other(),Piece::King){return;}
                moves.push(Move::new(from,to,if enemy & (1u64<<to)!=0 {CAPTURE} else {0}));
            });
        });
    }
    if side==Color::White && b.squares[4]==encode(side,Piece::King) {
        if b.castling&WK!=0 && b.squares[7]==encode(side,Piece::Rook)
            && b.squares[5]==0 && b.squares[6]==0
            && !b.is_attacked(4,Color::Black) && !b.is_attacked(5,Color::Black) && !b.is_attacked(6,Color::Black) {
            moves.push(Move::new(4,6,CASTLE));
        }
        if b.castling&WQ!=0 && b.squares[0]==encode(side,Piece::Rook)
            && b.squares[3]==0 && b.squares[2]==0 && b.squares[1]==0
            && !b.is_attacked(4,Color::Black) && !b.is_attacked(3,Color::Black) && !b.is_attacked(2,Color::Black) {
            moves.push(Move::new(4,2,CASTLE));
        }
    } else if side==Color::Black && b.squares[60]==encode(side,Piece::King) {
        if b.castling&BK!=0 && b.squares[63]==encode(side,Piece::Rook)
            && b.squares[61]==0 && b.squares[62]==0
            && !b.is_attacked(60,Color::White) && !b.is_attacked(61,Color::White) && !b.is_attacked(62,Color::White) {
            moves.push(Move::new(60,62,CASTLE));
        }
        if b.castling&BQ!=0 && b.squares[56]==encode(side,Piece::Rook)
            && b.squares[59]==0 && b.squares[58]==0 && b.squares[57]==0
            && !b.is_attacked(60,Color::White) && !b.is_attacked(59,Color::White) && !b.is_attacked(58,Color::White) {
            moves.push(Move::new(60,58,CASTLE));
        }
    }
    moves
}

pub fn legal_moves(board:&mut Board)->Vec<Move> {
    let side=board.stm;let mut legal=Vec::with_capacity(48);
    for m in pseudo_moves(board) {
        let undo=board.make(m);
        if !board.in_check(side) {legal.push(m);}
        board.unmake(m,undo);
    }
    legal
}

/// Accept UCI moves only when they match a generated legal move, including flags.
pub fn find_uci_move(board:&mut Board, text:&str)->Option<Move> {
    legal_moves(board).into_iter().find(|m|m.uci()==text.trim().to_ascii_lowercase())
}

pub fn perft(board:&mut Board, depth:u8)->u64 {
    if depth==0 {return 1;}
    let moves=legal_moves(board);
    if depth==1 {return moves.len() as u64;}
    let mut nodes=0;
    for m in moves {
        let undo=board.make(m);
        nodes+=perft(board,depth-1);
        board.unmake(m,undo);
    }
    nodes
}
