use crate::{attacks::tables, types::*, zobrist::keys};

pub const STARTPOS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
pub const WK: u8 = 1;
pub const WQ: u8 = 2;
pub const BK: u8 = 4;
pub const BQ: u8 = 8;

#[derive(Clone)]
pub struct Board {
    pub pieces: [[u64;6];2],
    pub occupancy: [u64;3], // white, black, both
    pub squares: [u8;64],
    pub stm: Color,
    pub castling: u8,
    pub ep: Option<u8>,
    pub halfmove: u16,
    pub fullmove: u16,
    pub key: u64,
    pub king_sq: [u8;2],
    pub history: Vec<u64>,
}

#[derive(Clone, Copy)]
pub struct Undo {
    captured: u8,
    castling: u8,
    ep: Option<u8>,
    halfmove: u16,
    fullmove: u16,
    key: u64,
}
impl Default for Board {
    fn default() -> Self { Self::from_fen(STARTPOS).expect("valid start position") }
}
impl Board {
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 || parts.len() > 6 { return Err("FEN must have 4–6 fields".into()); }
        let mut board = Self { pieces:[[0;6];2], occupancy:[0;3], squares:[0;64],
            stm:Color::White, castling:0, ep:None, halfmove:0, fullmove:1,
            key:0, king_sq:[0;2], history:Vec::new() };
        let ranks: Vec<&str> = parts[0].split('/').collect();
        if ranks.len()!=8 { return Err("FEN must contain eight ranks".into()); }
        let mut kings = [0;2];
        for (row, text) in ranks.iter().enumerate() {
            let mut file=0usize;
            for ch in text.chars() {
                if let Some(n) = ch.to_digit(10) {
                    if !(1..=8).contains(&n) { return Err("invalid FEN digit".into()); }
                    file += n as usize;
                } else {
                    let kind = match ch.to_ascii_lowercase() {
                        'p'=>Piece::Pawn,'n'=>Piece::Knight,'b'=>Piece::Bishop,
                        'r'=>Piece::Rook,'q'=>Piece::Queen,'k'=>Piece::King,
                        _=>return Err(format!("unknown FEN piece: {ch}")),
                    };
                    if file>=8 { return Err("rank has more than eight squares".into()); }
                    let color = if ch.is_ascii_uppercase() {Color::White} else {Color::Black};
                    let sq = ((7-row)*8+file) as u8;
                    board.put(sq,encode(color,kind));
                    if kind==Piece::King { kings[color.idx()] += 1; }
                    file+=1;
                }
                if file>8 { return Err("rank has more than eight squares".into()); }
            }
            if file!=8 { return Err("rank has fewer than eight squares".into()); }
        }
        if kings != [1,1] { return Err("FEN must contain one king per side".into()); }
        board.stm = match parts[1] {"w"=>Color::White,"b"=>Color::Black,_=>return Err("invalid side to move".into())};
        if parts[2]!="-" {
            for c in parts[2].chars() { board.castling |= match c { 'K'=>WK,'Q'=>WQ,'k'=>BK,'q'=>BQ,_=>return Err("invalid castling rights".into()) }; }
        }
        if parts[3]!="-" {
            board.ep = Some(parse_square(parts[3]).ok_or("invalid en-passant square")?);
            let ep_rank = board.ep.unwrap() / 8;
            if ep_rank != 2 && ep_rank != 5 { return Err("invalid en-passant rank".into()); }
        }
        if parts.len()>4 { board.halfmove=parts[4].parse().map_err(|_|"invalid halfmove clock")?; }
        if parts.len()>5 { board.fullmove=parts[5].parse().map_err(|_|"invalid fullmove counter")?; }
        board.key=board.compute_key();
        board.history.push(board.key);
        Ok(board)
    }

    pub fn to_fen(&self) -> String {
        let mut ranks=Vec::new();
        for rank in (0..8).rev() {
            let mut line=String::new(); let mut empty=0;
            for file in 0..8 {
                let code=self.squares[rank*8+file];
                if code==0 { empty+=1; continue; }
                if empty!=0 {line.push(char::from_digit(empty,10).unwrap());empty=0;}
                let (color,kind)=decode(code);
                let c=match kind {Piece::Pawn=>'p',Piece::Knight=>'n',Piece::Bishop=>'b',Piece::Rook=>'r',Piece::Queen=>'q',Piece::King=>'k'};
                line.push(if color==Color::White {c.to_ascii_uppercase()} else {c});
            }
            if empty>0 {line.push(char::from_digit(empty,10).unwrap());}
            ranks.push(line);
        }
        let mut castles=String::new();
        for (right,ch) in [(WK,'K'),(WQ,'Q'),(BK,'k'),(BQ,'q')] {if self.castling & right != 0 {castles.push(ch);}}
        if castles.is_empty(){castles.push('-');}
        format!("{} {} {} {} {} {}",ranks.join("/"),if self.stm==Color::White {"w"} else {"b"},castles,
            self.ep.map(square_name).unwrap_or_else(||"-".into()),self.halfmove,self.fullmove)
    }

    #[inline] fn put(&mut self, sq:u8, code:u8) {
        debug_assert_eq!(self.squares[sq as usize],0);
        let (color,kind)=decode(code);
        let bit=1u64<<sq;
        self.pieces[color.idx()][kind.idx()] |= bit;
        self.occupancy[color.idx()] |= bit;
        self.occupancy[2] |= bit;
        self.squares[sq as usize]=code;
        self.key ^= keys().piece[color.idx()][kind.idx()][sq as usize];
        if kind==Piece::King {self.king_sq[color.idx()]=sq;}
    }
    #[inline] fn take(&mut self, sq:u8) -> u8 {
        let code=self.squares[sq as usize];
        debug_assert_ne!(code,0);
        let (color,kind)=decode(code);let bit=1u64<<sq;
        self.pieces[color.idx()][kind.idx()] &= !bit;
        self.occupancy[color.idx()] &= !bit;
        self.occupancy[2] &= !bit;
        self.squares[sq as usize]=0;
        self.key ^= keys().piece[color.idx()][kind.idx()][sq as usize];
        code
    }
    fn ep_file_for_key(&self)->Option<usize> {
        let ep=self.ep?;
        // Ignore phantom EP squares in repetition keys.
        let pawn_rank = if self.stm==Color::White {ep.checked_sub(8)?} else {ep.checked_add(8)?};
        let file=ep%8;
        for df in [-1i8,1] {
            let f=file as i8+df;
            if (0..8).contains(&f) && self.squares[(pawn_rank/8*8 + f as u8) as usize]==encode(self.stm,Piece::Pawn) {
                return Some(file as usize);
            }
        }
        None
    }
    fn compute_key(&self) -> u64 {
        let z=keys();let mut key=0;
        for (sq,&code) in self.squares.iter().enumerate() {
            if code!=0 { let (c,p)=decode(code);key ^= z.piece[c.idx()][p.idx()][sq]; }
        }
        key ^= z.castle[self.castling as usize];
        if let Some(f)=self.ep_file_for_key() {key ^= z.en_passant[f];}
        if self.stm==Color::Black {key ^= z.black_to_move;}
        key
    }

    #[inline] pub fn is_attacked(&self, sq:u8, by:Color) -> bool {
        let t=tables();let p=&self.pieces[by.idx()];let occ=self.occupancy[2];
        (t.pawn(by.other().idx(),sq)&p[Piece::Pawn.idx()] != 0)
        || (t.knight(sq)&p[Piece::Knight.idx()] != 0)
        || (t.king(sq)&p[Piece::King.idx()] != 0)
        || (t.bishop(sq,occ)&(p[Piece::Bishop.idx()]|p[Piece::Queen.idx()]) != 0)
        || (t.rook(sq,occ)&(p[Piece::Rook.idx()]|p[Piece::Queen.idx()]) != 0)
    }
    #[inline] pub fn in_check(&self,color:Color)->bool {self.is_attacked(self.king_sq[color.idx()],color.other())}

    pub fn make(&mut self, m:Move)->Undo {
        let side=self.stm;
        let moved=self.squares[m.from as usize];
        debug_assert_ne!(moved,0);
        let (_,kind)=decode(moved);
        let captured = if m.flags & EN_PASSANT != 0 {
            let victim=(m.to as i16-side.pawn_step() as i16) as u8;
            self.squares[victim as usize]
        } else {self.squares[m.to as usize]};
        let undo=Undo {captured,castling:self.castling,ep:self.ep,halfmove:self.halfmove,fullmove:self.fullmove,key:self.key};
        let z=keys();
        self.key ^= z.castle[self.castling as usize];
        if let Some(f)=self.ep_file_for_key() { self.key ^= z.en_passant[f]; }
        self.ep=None;
        self.take(m.from);
        if captured!=0 {
            let victim=if m.flags & EN_PASSANT != 0 {(m.to as i16-side.pawn_step() as i16) as u8} else {m.to};
            self.take(victim);
            self.castling &= match victim {0=>!WQ,7=>!WK,56=>!BQ,63=>!BK,_=>15};
        }
        if kind==Piece::King {
            self.castling &= if side==Color::White {! (WK|WQ)} else {!(BK|BQ)};
        } else if kind==Piece::Rook {
            self.castling &= match m.from {0=>!WQ,7=>!WK,56=>!BQ,63=>!BK,_=>15};
        }
        self.put(m.to,encode(side,m.promotion.unwrap_or(kind)));
        if m.flags & CASTLE != 0 {
            let (rook_from,rook_to)=match m.to {6=>(7,5),2=>(0,3),62=>(63,61),58=>(56,59),_=>unreachable!()};
            let rook=self.take(rook_from);
            self.put(rook_to,rook);
        }
        if m.flags & DOUBLE_PUSH != 0 {
            self.ep=Some((m.from as i16+side.pawn_step() as i16) as u8);
        }
        self.halfmove = if kind==Piece::Pawn || captured!=0 {0} else {self.halfmove.saturating_add(1)};
        if side==Color::Black {self.fullmove=self.fullmove.saturating_add(1);}
        self.stm=side.other();
        self.key ^= z.castle[self.castling as usize] ^ z.black_to_move;
        if let Some(f)=self.ep_file_for_key() {self.key ^= z.en_passant[f];}
        self.history.push(self.key);
        undo
    }

    pub fn unmake(&mut self,m:Move,u:Undo) {
        self.history.pop();
        self.stm=self.stm.other();
        let moved=self.take(m.to);
        let (_,kind)=decode(moved);
        self.put(m.from,encode(self.stm,if m.promotion.is_some() {Piece::Pawn} else {kind}));
        if m.flags & CASTLE != 0 {
            let (rook_from,rook_to)=match m.to {6=>(7,5),2=>(0,3),62=>(63,61),58=>(56,59),_=>unreachable!()};
            let rook=self.take(rook_to);
            self.put(rook_from,rook);
        }
        if u.captured!=0 {
            let victim=if m.flags & EN_PASSANT != 0 {(m.to as i16-self.stm.pawn_step() as i16) as u8} else {m.to};
            self.put(victim,u.captured);
        }
        self.castling=u.castling;self.ep=u.ep;self.halfmove=u.halfmove;self.fullmove=u.fullmove;self.key=u.key;
    }

    pub fn is_repetition(&self)->bool {
        let len=self.history.len();
        if len<5{return false;}
        let back=self.halfmove as usize;
        let mut found=0;
        let mut i=len-3; // same side to move, two plies back
        loop {
            if self.history[i]==self.key {found+=1;if found>=2 {return true;}}
            if i<2 || len-1-i>=back {break;}
            i-=2;
        }
        false
    }
}
