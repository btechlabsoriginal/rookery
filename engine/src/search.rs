use std::{mem::size_of, sync::{Arc,atomic::{AtomicBool,Ordering}}, time::Duration};
use web_time::Instant;
use crate::{board::Board,eval::evaluate,movegen::{find_uci_move,legal_moves},types::*};

const INF:i32=32_000;
const MATE:i32=30_000;
const MATE_THRESHOLD:i32=29_000;
const MAX_PLY:i32=96;
const PIECE_VALUE:[i32;6]=[100,320,330,500,900,20_000];

#[derive(Clone,Copy,Default,Debug)]
enum Bound { #[default] Exact, Lower, Upper }
#[derive(Clone,Copy,Default)]
struct Entry { key:u64, best:Move, score:i32, depth:i16, bound:Bound, age:u8 }

#[derive(Clone,Copy,Debug)]
pub struct Limits {
    pub depth:u8,
    pub time:Option<Duration>,
    pub nodes:Option<u64>,
}
impl Default for Limits {
    fn default()->Self {Self{depth:8,time:None,nodes:None}}
}

#[derive(Clone,Debug)]
pub struct Analysis {
    pub depth:u8,
    pub score_cp:i32,
    pub nodes:u64,
    pub elapsed:Duration,
    pub pv:Vec<Move>,
    pub best_move:Option<Move>,
}
impl Analysis {
    pub fn uci_info(&self)->String {
        let score=if self.score_cp.abs()>=MATE_THRESHOLD {
            let plies=MATE-self.score_cp.abs();
            let moves=(plies+1)/2;
            format!("mate {}",if self.score_cp>0 {moves} else {-moves})
        } else {format!("cp {}",self.score_cp)};
        let ms=self.elapsed.as_millis().max(1);
        let nps=(self.nodes as u128*1000/ms) as u64;
        let pv=self.pv.iter().map(|m|m.uci()).collect::<Vec<_>>().join(" ");
        format!("info depth {} score {} nodes {} time {} nps {} pv {}",self.depth,score,self.nodes,ms,nps,pv)
    }
}

pub struct Searcher {
    tt:Vec<Entry>,
    age:u8,
    nodes:u64,
    aborted:bool,
    stop:Option<Arc<AtomicBool>>,
    deadline:Option<Instant>,
    max_nodes:Option<u64>,
    killers:[[Move;2];MAX_PLY as usize],
    history:[[i32;64];12],
}
impl Searcher {
    pub fn new(hash_mb:usize)->Self {
        let entries=(hash_mb.max(1)*1024*1024/size_of::<Entry>()).max(1);
        Self {tt:vec![Entry::default();entries],age:0,nodes:0,aborted:false,stop:None,
            deadline:None,max_nodes:None,killers:[[Move::default();2];MAX_PLY as usize],history:[[0;64];12]}
    }
    pub fn set_stop_flag(&mut self,stop:Arc<AtomicBool>) {self.stop=Some(stop);}
    pub fn clear_hash(&mut self) {self.tt.fill(Entry::default());}
    #[inline] fn index(&self,key:u64)->usize {(key as usize)%self.tt.len()}
    #[inline] fn stopped(&mut self)->bool {
        if self.nodes & 2047 != 0 {return false;}
        let stop=self.stop.as_ref().map(|s|s.load(Ordering::Relaxed)).unwrap_or(false)
            || self.deadline.map(|d|Instant::now()>=d).unwrap_or(false)
            || self.max_nodes.map(|n|self.nodes>=n).unwrap_or(false);
        self.aborted |=stop;
        stop
    }
    fn move_score(&self,b:&Board,m:Move,tt_move:Option<Move>,ply:usize)->i32 {
        if Some(m)==tt_move{return 1_000_000;}
        let attacker=decode(b.squares[m.from as usize]).1;
        if m.is_capture() {
            let victim=if m.flags&EN_PASSANT!=0 {Piece::Pawn} else {decode(b.squares[m.to as usize]).1};
            return 20_000+16*PIECE_VALUE[victim.idx()]-PIECE_VALUE[attacker.idx()] / 10;
        }
        if let Some(p)=m.promotion {return 10_000+PIECE_VALUE[p.idx()];}
        if ply<MAX_PLY as usize {
            if self.killers[ply][0]==m {return 9_000;}
            if self.killers[ply][1]==m {return 8_000;}
        }
        self.history[decode(b.squares[m.from as usize]).0.idx()*6+attacker.idx()][m.to as usize]
    }
    fn sorted_moves(&self,b:&Board,mut moves:Vec<Move>,tt:Option<Move>,ply:usize)->Vec<Move> {
        moves.sort_unstable_by_key(|m|-self.move_score(b,*m,tt,ply));
        moves
    }
    fn store(&mut self,key:u64,depth:i16,score:i32,ply:i32,bound:Bound,best:Move) {
        let i=self.index(key);let old=self.tt[i];
        if old.key!=key || depth>=old.depth || old.age!=self.age {
            let adjusted=if score>=MATE_THRESHOLD {score+ply} else if score<=-MATE_THRESHOLD {score-ply} else {score};
            self.tt[i]=Entry {key,best,score:adjusted,depth,bound,age:self.age};
        }
    }
    #[inline] fn unpack(score:i32,ply:i32)->i32 {
        if score>=MATE_THRESHOLD {score-ply} else if score<=-MATE_THRESHOLD {score+ply} else {score}
    }
    fn quiescence(&mut self,b:&mut Board,mut alpha:i32,beta:i32,ply:i32)->i32 {
        self.nodes+=1;
        if self.stopped() {return 0;}
        if b.halfmove>=100 || b.is_repetition() {return 0;}
        if ply>=MAX_PLY {return evaluate(b);}
        let in_check=b.in_check(b.stm);
        if !in_check {
            let stand=evaluate(b);
            if stand>=beta {return beta;}
            if stand>alpha {alpha=stand;}
        }
        let legal=legal_moves(b);
        let moves=self.sorted_moves(b,legal,None,ply as usize);
        let mut searched=0;
        for m in moves {
            if !in_check && !m.is_capture() && m.promotion.is_none(){continue;}
            searched+=1;
            let u=b.make(m);
            let score=-self.quiescence(b,-beta,-alpha,ply+1);
            b.unmake(m,u);
            if self.aborted{return 0;}
            if score>=beta {return beta;}
            if score>alpha {alpha=score;}
        }
        if in_check && searched==0 {-MATE+ply} else {alpha}
    }
    fn negamax(&mut self,b:&mut Board,depth:i32,mut alpha:i32,beta:i32,ply:i32)->i32 {
        self.nodes+=1;
        if self.stopped() {return 0;}
        if b.halfmove>=100 || b.is_repetition() {return 0;}
        if depth<=0 {return self.quiescence(b,alpha,beta,ply);}
        if ply>=MAX_PLY {return evaluate(b);}
        let alpha_orig=alpha;let key=b.key;let entry=self.tt[self.index(key)];
        let tt_best=if entry.key==key {Some(entry.best)} else {None};
        if entry.key==key && entry.depth>=depth as i16 && ply>0 {
            let score=Self::unpack(entry.score,ply);
            match entry.bound {
                Bound::Exact=>return score,
                Bound::Lower if score>=beta=>return score,
                Bound::Upper if score<=alpha=>return score,
                _=>{},
            }
        }
        let legal=legal_moves(b);
        let moves=self.sorted_moves(b,legal,tt_best,ply as usize);
        if moves.is_empty() {return if b.in_check(b.stm){-MATE+ply}else{0};}
        let mut best=-INF;let mut best_move=Move::default();
        for m in moves {
            let u=b.make(m);
            let score=-self.negamax(b,depth-1,-beta,-alpha,ply+1);
            b.unmake(m,u);
            if self.aborted {return 0;}
            if score>best {best=score;best_move=m;}
            if score>alpha {alpha=score;}
            if alpha>=beta {
                if !m.is_capture() && m.promotion.is_none() && (ply as usize)<MAX_PLY as usize {
                    let p=ply as usize;
                    if self.killers[p][0]!=m {self.killers[p][1]=self.killers[p][0];self.killers[p][0]=m;}
                    let code=b.squares[m.from as usize];
                    let hist=&mut self.history[(code-1) as usize][m.to as usize];
                    *hist=(*hist+(depth*depth).min(128)).min(10_000);
                }
                break;
            }
        }
        let bound=if best<=alpha_orig {Bound::Upper} else if best>=beta {Bound::Lower} else {Bound::Exact};
        self.store(key,depth as i16,best,ply,bound,best_move);
        best
    }
    fn pv(&self,root:&Board,depth:usize)->Vec<Move> {
        let mut b=root.clone();let mut pv=Vec::new();
        let mut seen=std::collections::HashSet::new();
        for _ in 0..depth {
            if !seen.insert(b.key){break;}
            let e=self.tt[self.index(b.key)];if e.key!=b.key{break;}
            // Stored best may be from a bound or a collision; validate it.
            if let Some(m)=find_uci_move(&mut b,&e.best.uci()) {
                b.make(m);pv.push(m);
            } else {break;}
        }
        pv
    }
    pub fn search(&mut self,b:&mut Board,limits:Limits)->Analysis {
        self.search_with_info(b,limits,|_|{})
    }
    pub fn search_with_info(&mut self,b:&mut Board,limits:Limits,mut info:impl FnMut(&Analysis))->Analysis {
        self.age=self.age.wrapping_add(1);self.nodes=0;self.aborted=false;
        self.killers=[[Move::default();2];MAX_PLY as usize];
        self.history=[[0;64];12];
        self.max_nodes=limits.nodes;
        let start=Instant::now();
        self.deadline=limits.time.map(|duration|start+duration);
        let fallback=legal_moves(b).into_iter().next();
        let mut last=Analysis {depth:0,score_cp:0,nodes:0,elapsed:Duration::ZERO,pv:vec![],best_move:fallback};
        if fallback.is_none() {return last;}
        for depth in 1..=limits.depth.max(1) {
            let score=self.negamax(b,depth as i32,-INF,INF,0);
            if self.aborted {break;}
            let pv=self.pv(b,depth as usize);
            let best=pv.first().copied().or(last.best_move);
            last=Analysis{depth,score_cp:score,nodes:self.nodes,elapsed:start.elapsed(),pv,best_move:best};
            info(&last);
            if score.abs()>=MATE_THRESHOLD {break;}
        }
        last
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn mate_in_one() {
        let mut b=Board::from_fen("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1").unwrap();
        let result=Searcher::new(1).search(&mut b,Limits{depth:3,..Limits::default()});
        assert!(result.score_cp >= MATE_THRESHOLD);
        assert!(result.best_move.is_some());
    }
}
