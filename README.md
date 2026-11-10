# Rookery

A bitboard chess engine in Rust with a UCI command-line executable and a WebAssembly-powered React interface. The Rust crates handle chess rules, evaluation, and search; the frontend handles presentation and input.

**What's here**

| Directory | Purpose |
| --- | --- |
| `engine/` | Core engine, bitboards, magic sliding attacks, board state, legal moves, tapered eval, TT and search; native UCI executable |
| `wasm/` | `wasm-bindgen` bridge: `best_move(fen, depth)` and `analyse(fen, depth)` |
| `web/` | Vite + React interface, HTML drag-and-drop, tap-to-move, history, promotion picker, search depth, analysis and engine opponent |

## Requirements

- Stable Rust and Cargo (recommended: install via [rustup](https://rustup.rs/))
- The `wasm32-unknown-unknown` Rust target
- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/)
- Node.js 20+ and npm

## Screenshots

<img width="1440" height="900" alt="Screenshot 2026-11-10 at 19 42 41" src="https://github.com/user-attachments/assets/b322379a-4374-45ee-8304-dbd1a697eeb8" />

## 1. Native engine (UCI)

From the repository root:

```sh
cargo build --release -p rookery-engine
./target/release/rookery
```

Example commands after starting the program:

```text
uci
isready
position startpos moves e2e4 e7e5 g1f3
setoption name Hash value 32
go depth 6
stop
quit
```

The executable also understands `position fen <six FEN fields> moves ...`, `ucinewgame`, `go movetime 2000`, `go wtime ... btime ... winc ... binc ...`, `go nodes ...`, and `go perft 5`. Search happens on a worker thread so the UCI command reader can answer `stop` and `isready` during a search. Search output uses standard `info depth ... score cp ... nodes ... pv ...` and `bestmove` lines.

To connect Rookery to a desktop chess GUI, choose **UCI engine** and select the `rookery` executable.

## 2. WebAssembly

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack  # skip if already installed
wasm-pack build wasm --release --target web --out-dir ../web/src/wasm-bindings
```

> `--out-dir` is relative to the `wasm/` crate. Run the command from the workspace root as shown. The generated bindings are intentionally ignored by Git; rebuild them when Rust sources change.

The exported `best_move(fen, depth)` returns a UCI string such as `e2e4`, or `0000` if the position has no legal move. The `analyse(fen, depth)` function returns JSON containing `bestMove`, `scoreCp`, `depth`, `nodes`, `elapsedMs`, and `pv` (UCI move strings). Scores are always from the side-to-move's perspective.

## 3. Browser interface

After compiling the WASM bindings:

```sh
cd web
npm install
npm run dev
```

Open the address printed by Vite (usually http://localhost:5173). To make a static production bundle:

```sh
npm run build
npm run preview
```

The search runs inside `engine.worker.js`, not React's main thread. Moving pieces, changing the position, or starting a new game cancels stale work by terminating an in-progress worker. **Play engine** makes Rookery play the opposite colour; **Analyse freely** lets you move either colour and automatically requests a line. Select a piece and tap its destination or drag-and-drop. Promotions open a piece picker.

## 4. Verify move generation

```sh
cargo test -p rookery-engine
```

Perft uses the *legal* move generator, including make/unmake, check filtering, castling-through-check rejection, en passant, and all four promotions. Golden numbers come from the standard chessprogramming.org/Perft_Results suite:

| Test position | D1 | D2 | D3 | D4 | D5 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Starting position | 20 | 400 | 8,902 | 197,281 | 4,865,609 |
| Kiwipete | 48 | 2,039 | 97,862 | 4,085,603 | 193,690,690† |
| Position 3 | 14 | 191 | 2,812 | 43,238 | 674,624 |

† Kiwipete at depth 5 is an ignored-by-default test because of its 193 million nodes; execute it explicitly:

```sh
cargo test -p rookery-engine kiwipete_depth_five --release -- --ignored
```

The suite also exercises positions 4/5, reversibility of castling/en passant/promotion, and exhaustive magic-attack-versus-ray checks for every sliding occupancy subset.

## Implementation notes

- **Board:** `[[u64; 6]; 2]` piece bitboards, three occupancy masks, a small mailbox for direct piece lookup, an incremental 64-bit Zobrist key, castling/EP state, and a reversible history. Squares index from `a1 = 0` to `h8 = 63`.
- **Sliders:** Deterministically generated *offline* magic multipliers, stored in `engine/src/attacks/magic_numbers.rs`; on first use, build lookup tables for every blocker subset. No dependency on BMI2/PEXT.
- **Move generation:** Pseudo moves first, then make/unmake and king-in-check rejection; castling transit squares are checked before committing. Correctness takes priority over aggressively specialized pin masks.
- **Search:** Iterative-deepening negamax/alpha-beta, transposition-table bound types, mate-distance normalization, MVV-LVA captures, killer/history heuristics, quiescence (all evasions when checked), repetition and 50-move detection. `Hash` is UCI-configurable per search.
- **Evaluation:** Middlegame/endgame interpolation over non-pawn material phase, material values, procedural piece-square tables, mobility, and king-ring danger / pawn shield.

### Known limitations

This is a playable hobby engine rather than a Stockfish competitor. It uses straightforward legal generation, a single search thread, no opening book or Syzygy tablebases, and no NNUE. Depth is capped at 6 plies in the browser so ordinary hardware stays usable; the native UCI engine can search much deeper. The WASM API is synchronous *inside its worker*, so cancelling a search replaces the worker. Search timing uses `web-time`, which also works in browser workers. Rust's initial WASM compilation and attack-table initialization can take a moment on the first search.

## License

MIT — see `LICENSE`.
