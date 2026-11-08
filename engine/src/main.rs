//! Single-threaded search, asynchronous UCI controller. 'stop' remains responsive
//! because search runs in its own thread and polls an atomic cancellation flag.
use std::{io::{self,BufRead,Write},sync::{Arc,atomic::{AtomicBool,Ordering}},thread,time::Duration};
use rookery_engine::{Board,Color,Limits,Searcher,find_uci_move,perft};

struct Active {stop:Arc<AtomicBool>,thread:thread::JoinHandle<()>}
fn stop_search(active:&mut Option<Active>){
    if let Some(job)=active.take(){job.stop.store(true,Ordering::Relaxed);let _=job.thread.join();}
}
fn set_position(args:&[&str],board:&mut Board) {
    if args.is_empty(){return;}
    let mut i=0;
    let rebuilt=if args[i]=="startpos" {i+=1;Ok(Board::default())}
    else if args[i]=="fen" {
        i+=1;let start=i;
        while i<args.len() && args[i]!="moves" {i+=1;}
        Board::from_fen(&args[start..i].join(" "))
    }else {Err("expected 'startpos' or 'fen'".to_owned())};
    match rebuilt {
        Ok(b)=>*board=b,
        Err(error)=>{println!("info string bad position: {error}");return;}
    }
    if args.get(i)==Some(&"moves") {i+=1;}
    for text in &args[i..] {
        match find_uci_move(board,text){
            Some(m)=>{board.make(m);},
            None=>{println!("info string illegal move: {text}");break;}
        }
    }
}
fn parse_go(args:&[&str],color:Color)->Limits {
    let mut limit=Limits{depth:64,time:None,nodes:None};
    let mut wtime=None;let mut btime=None;let mut winc=0u64;let mut binc=0u64;let mut movestogo=30u64;
    let mut i=0;
    while i<args.len(){
        let value=args.get(i+1).and_then(|v|v.parse::<u64>().ok());
        match args[i] {
            "depth"=>if let Some(n)=value{limit.depth=n.clamp(1,64) as u8;},
            "movetime"=>if let Some(n)=value{limit.time=Some(Duration::from_millis(n));},
            "nodes"=>limit.nodes=value,
            "wtime"=>wtime=value,"btime"=>btime=value,
            "winc"=>winc=value.unwrap_or(0),"binc"=>binc=value.unwrap_or(0),
            "movestogo"=>movestogo=value.unwrap_or(30).max(1),
            "infinite"=>limit.time=None,
            _=>{},
        }
        i+=if value.is_some(){2}else{1};
    }
    if limit.time.is_none() {
        let (clock,inc)=if color==Color::White {(wtime,winc)}else{(btime,binc)};
        if let Some(ms)=clock {
            let allowance=(ms/movestogo+inc/2).clamp(10,ms.saturating_sub(25).max(10));
            limit.time=Some(Duration::from_millis(allowance));
        }
    }
    limit
}
fn main(){
    let stdin=io::stdin();let mut board=Board::default();let mut hash_mb=16usize;
    let mut active:Option<Active>=None;
    for line in stdin.lock().lines(){
        let Ok(line)=line else {break};
        let parts:Vec<_>=line.split_whitespace().collect();
        if parts.is_empty(){continue;}
        match parts[0] {
            "uci"=>{
                println!("id name Rookery 0.1");println!("id author Rookery contributors");
                println!("option name Hash type spin default 16 min 1 max 512");
                println!("uciok");
            }
            "isready"=>println!("readyok"),
            "ucinewgame"=>{stop_search(&mut active);board=Board::default();},
            "setoption"=>{
                if let Some(pos)=parts.iter().position(|v|*v=="value") {
                    if parts.get(2)==Some(&"Hash") {if let Some(n)=parts.get(pos+1).and_then(|v|v.parse::<usize>().ok()){hash_mb=n.clamp(1,512);}}
                }
            }
            "position"=>{stop_search(&mut active);set_position(&parts[1..],&mut board);},
            "go"=>{
                stop_search(&mut active);
                if parts.get(1)==Some(&"perft") {
                    let depth=parts.get(2).and_then(|x|x.parse::<u8>().ok()).unwrap_or(4);
                    let mut b=board.clone();let nodes=perft(&mut b,depth);
                    println!("info string perft depth {depth} nodes {nodes}");
                    println!("bestmove 0000");continue;
                }
                let limits=parse_go(&parts[1..],board.stm);
                let mut current=board.clone();let stop=Arc::new(AtomicBool::new(false));
                let flag=stop.clone();let mb=hash_mb;
                let handle=thread::spawn(move ||{
                    let mut searcher=Searcher::new(mb);
                    searcher.set_stop_flag(flag);
                    let analysis=searcher.search_with_info(&mut current,limits,|a|{
                        println!("{}",a.uci_info());let _=io::stdout().flush();
                    });
                    println!("bestmove {}",analysis.best_move.map(|m|m.uci()).unwrap_or_else(||"0000".into()));
                    let _=io::stdout().flush();
                });
                active=Some(Active{stop,thread:handle});
            }
            "stop"=>stop_search(&mut active),
            "d"=>println!("info string {}",board.to_fen()),
            "quit"=>{stop_search(&mut active);break;},
            _=>{},
        }
        let _=io::stdout().flush();
    }
    stop_search(&mut active);
}
