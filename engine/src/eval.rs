//! White-relative middlegame/endgame evaluation, tapered by remaining material.
//! PSTs are produced from geometric features (centre, advancement, king exposure)
//! rather than maintaining 12 almost-duplicate 64-element lookup arrays.
use crate::{attacks::tables, board::Board, types::*};

const MG: [i32;6] = [100, 322, 335, 500, 950, 0];
const EG: [i32;6] = [120, 300, 320, 525, 940, 0];
const PHASE: [i32;6] = [0,1,1,2,4,0];
const MAX_PHASE:i32=24;

#[inline] fn center(f:i32,r:i32)->i32 {6-(2*f-7).abs()/2-(2*r-7).abs()/2}
#[inline] fn pst(p:Piece,sq:u8,color:Color,endgame:bool)->i32 {
    let f=(sq%8) as i32;
    let r=if color==Color::White {(sq/8) as i32} else {7-(sq/8) as i32};
    let c=center(f,r);
    match p {
        Piece::Pawn=>if endgame {r*11 + c*3} else {r*7 + c*3},
        Piece::Knight=>c*10 - if r==0 {15} else {0},
        Piece::Bishop=>c*5 + r*2,
        Piece::Rook=>r*3 + if r==6 {16} else {0},
        Piece::Queen=>c*2,
        Piece::King=>if endgame {c*9} else {-(r*9) - c*7 + if f==1 || f==6 {12} else {0}},
    }
}

fn score_side(board:&Board,side:Color)->(i32,i32,i32) {
    let mut mg=0;let mut eg=0;let mut phase=0;
    let t=tables();let occupied=board.occupancy[2];let friendly=board.occupancy[side.idx()];
    for piece in Piece::ALL {
        let mut bb=board.pieces[side.idx()][piece.idx()];
        let count=bb.count_ones() as i32;
        phase+=PHASE[piece.idx()]*count;
        while bb!=0 {
            let sq=bb.trailing_zeros() as u8;bb&=bb-1;
            mg+=MG[piece.idx()]+pst(piece,sq,side,false);
            eg+=EG[piece.idx()]+pst(piece,sq,side,true);
            // Mobility rewards legal-looking reach (pins are handled by search).
            let reach=match piece {
                Piece::Knight=>t.knight(sq),
                Piece::Bishop=>t.bishop(sq,occupied),
                Piece::Rook=>t.rook(sq,occupied),
                Piece::Queen=>t.queen(sq,occupied),_=>0,
            } & !friendly;
            let weight=match piece {Piece::Knight=>4,Piece::Bishop=>4,Piece::Rook=>2,Piece::Queen=>1,_=>0};
            mg+=reach.count_ones() as i32*weight;
            eg+=reach.count_ones() as i32*(weight+1)/2;
        }
    }
    // King safety: a pawn shield and danger near the king's surrounding ring.
    let king=board.king_sq[side.idx()];
    let shield=t.king(king) & board.pieces[side.idx()][Piece::Pawn.idx()];
    mg+=shield.count_ones() as i32*11;
    let mut ring=t.king(king);
    while ring!=0 {
        let sq=ring.trailing_zeros() as u8;ring&=ring-1;
        if board.is_attacked(sq,side.other()) {mg-=7;}
    }
    (mg,eg,phase)
}

/// Positive numbers favour the side to move.
pub fn evaluate(board:&Board)->i32 {
    let (wm,we,wp)=score_side(board,Color::White);
    let (bm,be,bp)=score_side(board,Color::Black);
    let phase=(wp+bp).min(MAX_PHASE);
    let white=((wm-bm)*phase+(we-be)*(MAX_PHASE-phase))/MAX_PHASE;
    let tempo=12;
    if board.stm==Color::White {white+tempo} else {-white+tempo}
}
