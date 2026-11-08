import { useEffect, useRef, useState } from 'react';
import { Chess } from 'chess.js';

const PIECES = {
  wk: '♔', wq: '♕', wr: '♖', wb: '♗', wn: '♘', wp: '♙',
  bk: '♚', bq: '♛', br: '♜', bb: '♝', bn: '♞', bp: '♟',
};
const FILES = 'abcdefgh';
const PROMOTIONS = ['q', 'r', 'b', 'n'];

function describeGame(chess) {
  if (chess.isCheckmate()) return `${chess.turn() === 'w' ? 'Black' : 'White'} wins by checkmate`;
  if (chess.isStalemate()) return 'Draw by stalemate';
  if (chess.isThreefoldRepetition()) return 'Draw by threefold repetition';
  if (chess.isInsufficientMaterial()) return 'Draw · insufficient material';
  if (chess.isDraw()) return 'Draw';
  return `${chess.turn() === 'w' ? 'White' : 'Black'} to move${chess.isCheck() ? ' · Check!' : ''}`;
}

function prettyPv(fen, moves) {
  const board = new Chess(fen);
  const notation = [];
  for (const uci of moves ?? []) {
    try {
      const move = board.move({ from: uci.slice(0, 2), to: uci.slice(2, 4), promotion: uci[4] || 'q' });
      if (!move) break;
      notation.push(move.san);
    } catch { break; }
  }
  return notation.join(' ');
}

function MoveList({ history }) {
  const pairs = [];
  for (let i = 0; i < history.length; i += 2) {
    pairs.push({ number: i / 2 + 1, white: history[i]?.san, black: history[i + 1]?.san });
  }
  return (
    <div className="moves-scroll">
      {pairs.length === 0 ? (
        <div className="empty-history">Your game begins here.<span>Make a move to see the notation.</span></div>
      ) : (
        <div className="move-list">
          {pairs.map((row, index) => (
            <div className={`move-row ${index === pairs.length - 1 ? 'latest-row' : ''}`} key={row.number}>
              <span className="move-number">{row.number}.</span>
              <span>{row.white}</span><span>{row.black || '—'}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function Board({ chess, flipped, selected, legalTargets, lastMove, onSquare, onDrag, onDrop, canMove }) {
  const ranks = flipped ? [0,1,2,3,4,5,6,7] : [7,6,5,4,3,2,1,0];
  const files = flipped ? [7,6,5,4,3,2,1,0] : [0,1,2,3,4,5,6,7];
  const checkedKing = chess.isCheck() ? chess.board().flat().find(p => p?.type === 'k' && p.color === chess.turn())?.square : null;
  return (
    <div className="board-frame">
      <div className="board" role="group" aria-label="Chess board">
        {ranks.flatMap((rank, rIndex) => files.map((file, fIndex) => {
          const square = `${FILES[file]}${rank + 1}`;
          const piece = chess.get(square);
          const dark = (file + rank) % 2 === 0;
          const isSelected = selected === square;
          const legal = legalTargets.includes(square);
          const moved = lastMove.includes(square);
          const movable = canMove && piece?.color === chess.turn();
          return (
            <button
              type="button"
              key={square}
              className={['square', dark ? 'dark' : 'light', moved ? 'last-move' : '', isSelected ? 'selected' : '', checkedKing === square ? 'check' : ''].join(' ')}
              onClick={() => onSquare(square)}
              draggable={!!movable}
              onDragStart={event => onDrag(event, square)}
              onDragOver={event => event.preventDefault()}
              onDrop={event => { event.preventDefault(); onDrop(event, square); }}
              aria-label={`${square}${piece ? ` ${piece.color === 'w' ? 'white' : 'black'} ${piece.type}` : ''}`}
              aria-pressed={isSelected}
            >
              {fIndex === 0 && <span className="rank-coord">{rank + 1}</span>}
              {rIndex === 7 && <span className="file-coord">{FILES[file]}</span>}
              {piece && <span className={`piece ${piece.color === 'w' ? 'white-piece' : 'black-piece'}`}>{PIECES[piece.color + piece.type]}</span>}
              {legal && <span className={piece ? 'target-capture' : 'target-dot'} />}
            </button>
          );
        }))}
      </div>
    </div>
  );
}

export default function App() {
  const game = useRef(new Chess());
  const worker = useRef(null);
  const activeRequest = useRef(0);
  const workerBusy = useRef(false);
  const [fen, setFen] = useState(game.current.fen());
  const [selected, setSelected] = useState(null);
  const [promotion, setPromotion] = useState(null);
  const [lastMove, setLastMove] = useState([]);
  const [flipped, setFlipped] = useState(false);
  const [depth, setDepth] = useState(4);
  const [mode, setMode] = useState('play');
  const [humanSide, setHumanSide] = useState('w');
  const [analysis, setAnalysis] = useState(null);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);

  function buildWorker() {
    const instance = new Worker(new URL('./engine.worker.js', import.meta.url), { type: 'module' });
    instance.onmessage = ({ data }) => {
      if (data.id !== activeRequest.current || data.fen !== game.current.fen()) return;
      workerBusy.current = false;
      setBusy(false);
      if (data.error) { setError(`Engine: ${data.error}`); return; }
      setError('');
      setAnalysis({ ...data.report, fen: data.fen });
      if (data.play && data.report.bestMove) {
        const move = data.report.bestMove;
        applyMove(move.slice(0,2), move.slice(2,4), move[4] || 'q');
      }
    };
    return instance;
  }

  useEffect(() => {
    worker.current = buildWorker();
    return () => { activeRequest.current++; worker.current?.terminate(); worker.current = null; };
  }, []);

  function stopCurrent() {
    activeRequest.current++;
    if (workerBusy.current && worker.current) {
      worker.current.terminate();
      worker.current = buildWorker();
    }
    workerBusy.current = false;
    setBusy(false);
  }

  function requestEngine(position, play) {
    if (!worker.current) return;
    stopCurrent();
    const id = ++activeRequest.current;
    workerBusy.current = true;
    setBusy(true);
    setError('');
    worker.current.postMessage({ id, fen: position, depth, play });
  }

  useEffect(() => {
    if (game.current.isGameOver()) { stopCurrent(); return; }
    requestEngine(fen, mode === 'play' && game.current.turn() !== humanSide);
    return () => { activeRequest.current++; };
  }, [fen, mode, humanSide, depth]);

  const chess = game.current;
  const history = chess.history({ verbose: true });
  const canMove = !chess.isGameOver() && (mode === 'analysis' || chess.turn() === humanSide) && !promotion;
  const legalTargets = selected ? chess.moves({ square: selected, verbose: true }).map(m => m.to) : [];

  function applyMove(from, to, promoteTo = 'q') {
    try {
      const move = game.current.move({ from, to, promotion: promoteTo });
      if (!move) return false;
      setFen(game.current.fen());
      setLastMove([from, to]);
      setSelected(null);
      setPromotion(null);
      setAnalysis(null);
      return true;
    } catch { return false; }
  }

  function attemptMove(from, to) {
    if (!canMove) return false;
    const piece = chess.get(from);
    if (!piece || piece.color !== chess.turn()) return false;
    if (piece.type === 'p' && (to.endsWith('8') || to.endsWith('1')) && chess.moves({ square: from, verbose: true }).some(m => m.to === to && m.isPromotion())) {
      setPromotion({ from, to, color: piece.color });
      return true;
    }
    return applyMove(from, to);
  }

  function handleSquare(square) {
    if (!canMove) return;
    const piece = chess.get(square);
    if (selected && selected !== square && attemptMove(selected, square)) return;
    setSelected(piece && piece.color === chess.turn() ? square : null);
  }

  function onDrag(event, square) {
    event.dataTransfer.setData('text/plain', square);
    event.dataTransfer.effectAllowed = 'move';
  }
  function onDrop(event, square) {
    const from = event.dataTransfer.getData('text/plain');
    if (from && from !== square) attemptMove(from, square);
  }

  function newGame(side = humanSide) {
    stopCurrent();game.current = new Chess();setFen(game.current.fen());
    setHumanSide(side);setFlipped(side === 'b');setSelected(null);
    setPromotion(null);setAnalysis(null);setLastMove([]);setError('');
  }
  function undo() {
    stopCurrent();
    const last = game.current.undo();
    if (!last) return;
    if (mode === 'play' && game.current.history().length) game.current.undo();
    const remaining = game.current.history({ verbose: true });
    const previous = remaining.at(-1);
    setFen(game.current.fen());setLastMove(previous ? [previous.from, previous.to] : []);
    setSelected(null);setPromotion(null);setAnalysis(null);
  }
  function selectMode(next) {
    stopCurrent(); setMode(next);setAnalysis(null);setSelected(null);
  }

  const label = describeGame(chess);
  const topColor = flipped ? 'w' : 'b';
  const bottomColor = flipped ? 'b' : 'w';
  const topIsEngine = mode === 'play' && topColor !== humanSide;
  const bottomIsEngine = mode === 'play' && bottomColor !== humanSide;
  const engineTurn = mode === 'play' && chess.turn() !== humanSide && !chess.isGameOver();
  const pvText = analysis ? prettyPv(analysis.fen, analysis.pv) : '';
  const whiteScore = analysis ? analysis.scoreCp * (analysis.fen.split(' ')[1] === 'w' ? 1 : -1) / 100 : null;
  const scoreLabel = whiteScore == null ? '—' : `${whiteScore > 0 ? '+' : ''}${whiteScore.toFixed(2)}`;

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="logo"><div className="logo-mark">♜</div><div><strong>ROOKERY</strong><span>CHESS ENGINE / 0.1</span></div></div>
        <div className="top-right"><span className="status-pulse" /> <span>Rust · WebAssembly</span><span className="top-divider" /> <span>MIT licensed</span></div>
      </header>

      <main className="main-grid">
        <section className="game-column" aria-label="Chess game">
          <div className="section-eyebrow"><span className="tiny-rule" /> THE BOARD <span className="section-num">01 / 02</span></div>
          <div className="opponent-row">
            <div className="avatar">{topColor === 'w' ? '♔' : '♚'}</div>
            <div className="player-info"><strong>{mode === 'play' ? (topIsEngine ? 'Rookery Engine' : 'You') : `${topColor === 'w' ? 'White' : 'Black'} pieces`}</strong><span>{topIsEngine ? `Search depth ${depth}` : mode === 'play' ? `Playing ${topColor === 'w' ? 'White' : 'Black'}` : 'Analysis mode'}</span></div>
            <span className="engine-chip">{topIsEngine ? 'ENGINE' : mode === 'play' ? 'HUMAN' : 'BOARD'}</span>
          </div>
          <Board chess={chess} flipped={flipped} selected={selected} legalTargets={legalTargets} lastMove={lastMove} onSquare={handleSquare} onDrag={onDrag} onDrop={onDrop} canMove={canMove} />
          <div className="opponent-row bottom-player">
            <div className={`avatar ${bottomIsEngine ? '' : 'human-avatar'}`}>{bottomColor === 'w' ? '♔' : '♚'}</div>
            <div className="player-info"><strong>{mode === 'play' ? (bottomIsEngine ? 'Rookery Engine' : 'You') : `${bottomColor === 'w' ? 'White' : 'Black'} pieces`}</strong><span>{bottomIsEngine ? `Search depth ${depth}` : mode === 'play' ? `Playing ${bottomColor === 'w' ? 'White' : 'Black'}` : 'Both sides playable'}</span></div>
            <div className="game-state"><span className="small-dot" /> {engineTurn && busy ? 'Engine thinking' : label}</div>
          </div>
          <div className="action-bar">
            <button onClick={() => newGame()} className="action primary">＋ <span>New game</span></button>
            <button onClick={undo} className="action" disabled={!history.length}>↶ <span>Undo</span></button>
            <button onClick={() => setFlipped(v => !v)} className="action">⇅ <span>Flip board</span></button>
          </div>
        </section>

        <aside className="sidebar">
          <div className="section-eyebrow"><span className="tiny-rule" /> ENGINE ROOM <span className="section-num">02 / 02</span></div>
          <div className="panel engine-panel">
            <div className="panel-heading"><div><span className="overline">ANALYSIS</span><h2>Under the hood<span className="accent">.</span></h2></div><span className={`working-pill ${busy ? 'working' : ''}`}>{busy ? 'SEARCHING' : 'READY'}</span></div>
            <div className="mode-switch" role="group" aria-label="Game mode">
              <button className={mode === 'play' ? 'active' : ''} onClick={() => selectMode('play')}>Play engine</button>
              <button className={mode === 'analysis' ? 'active' : ''} onClick={() => selectMode('analysis')}>Analyse freely</button>
            </div>
            {mode === 'play' && <div className="color-choice"><span>Your pieces</span><div><button className={humanSide === 'w' ? 'chosen' : ''} onClick={() => newGame('w')}>♔ White</button><button className={humanSide === 'b' ? 'chosen' : ''} onClick={() => newGame('b')}>♚ Black</button></div></div>}
            <div className="depth-section"><div className="setting-head"><label htmlFor="depth">Search depth</label><strong>{depth} <small>plies</small></strong></div><input id="depth" type="range" min="1" max="6" step="1" value={depth} onChange={event => setDepth(Number(event.target.value))} /><div className="range-labels"><span>FASTER</span><span>STRONGER</span></div></div>
            <div className="analysis-output">
              <div className="eval-row"><div><span className="overline">EVALUATION</span><div className="score">{scoreLabel} <small>pawns (White)</small></div></div><div className="depth-reading"><span className="overline">COMPLETED</span><strong>{analysis?.depth ?? '—'}</strong><small>depth</small></div></div>
              <div className="score-meter"><div style={{ width: `${whiteScore === null ? 50 : Math.max(3,Math.min(97,50 + 45 * Math.tanh(whiteScore/3)))}%` }} /></div>
              <div className="principal-variation"><div className="pv-header"><span className="overline">PRINCIPAL VARIATION</span>{analysis && <span>{analysis.nodes.toLocaleString()} nodes</span>}</div><p>{busy ? 'Searching for the strongest continuation…' : pvText || 'Make a move to see the engine line.'}</p></div>
              {error && <p className="error-message">{error}</p>}
            </div>
            {mode === 'analysis' && <button className="analyse-button" disabled={busy || chess.isGameOver()} onClick={() => requestEngine(game.current.fen(),false)}>{busy ? 'Analysing…' : '↻  Analyse again'}</button>}
          </div>
          <div className="panel history-panel"><div className="history-head"><div><span className="overline">GAME RECORD</span><h3>Move history</h3></div><span className="count-chip">{history.length} moves</span></div><MoveList history={history} /></div>
          <div className="sidebar-foot">64 squares. Two sides. One search tree.<span>Rust + WebAssembly</span></div>
        </aside>
      </main>

      {promotion && (
        <div className="dialog-backdrop" role="presentation" onClick={() => setPromotion(null)}>
          <div className="promotion-dialog" role="dialog" aria-modal="true" aria-label="Choose promotion piece" onClick={e => e.stopPropagation()}>
            <span className="overline">PAWN PROMOTION</span><h3>Choose a piece</h3>
            <div className="promotion-choices">{PROMOTIONS.map(p => <button key={p} aria-label={`Promote to ${{q:'queen',r:'rook',b:'bishop',n:'knight'}[p]}`} onClick={() => applyMove(promotion.from,promotion.to,p)}>{PIECES[promotion.color+p]}</button>)}</div>
            <button className="cancel-promotion" onClick={() => setPromotion(null)}>Cancel</button>
          </div>
        </div>
      )}
    </div>
  );
}
