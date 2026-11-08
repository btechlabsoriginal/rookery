//! Browser-facing API. Analysis is intentionally synchronous inside a Web Worker:
//! the Rust search needn't know anything about React or the main UI thread.
use rookery_engine::{Board,Limits,Searcher};
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    best_move: Option<String>,
    depth: u8,
    score_cp: i32,
    nodes: u64,
    elapsed_ms: u128,
    pv: Vec<String>,
}

fn run(fen: &str, depth: u8) -> Result<Report, JsValue> {
    let mut board=Board::from_fen(fen).map_err(|e|JsValue::from_str(&e))?;
    let mut searcher=Searcher::new(8);
    let result=searcher.search(&mut board,Limits {depth:depth.clamp(1,12),..Limits::default()});
    Ok(Report {
        best_move:result.best_move.map(|m|m.uci()),
        depth:result.depth,
        score_cp:result.score_cp,
        nodes:result.nodes,
        elapsed_ms:result.elapsed.as_millis(),
        pv:result.pv.into_iter().map(|m|m.uci()).collect(),
    })
}

/// Returns the strongest legal move as a UCI string, or "0000" for a terminal position.
#[wasm_bindgen]
pub fn best_move(fen: &str, depth: u8) -> Result<String, JsValue> {
    Ok(run(fen,depth)?.best_move.unwrap_or_else(||"0000".to_string()))
}

/// JSON includes bestMove, scoreCp (side-to-move relative), depth, nodes and PV.
#[wasm_bindgen]
pub fn analyse(fen: &str, depth: u8) -> Result<String, JsValue> {
    serde_json::to_string(&run(fen,depth)?).map_err(|e|JsValue::from_str(&e.to_string()))
}
