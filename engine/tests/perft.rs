use rookery_engine::{Board,perft,find_uci_move};

fn verify(fen:&str,expected:&[u64]) {
    let mut b=Board::from_fen(fen).unwrap();
    let original=b.to_fen();let key=b.key;
    for (i,&nodes) in expected.iter().enumerate(){
        assert_eq!(perft(&mut b,(i+1) as u8),nodes,"depth {} at {fen}",i+1);
        assert_eq!(b.key,key,"hash not restored after perft");
        assert_eq!(b.to_fen(),original,"board not restored after perft");
    }
}

#[test]
fn start_position_depths_one_to_five() {
    verify(rookery_engine::STARTPOS,&[20,400,8_902,197_281,4_865_609]);
}
#[test]
fn kiwipete_castling_pins_checks() {
    verify("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &[48,2039,97_862,4_085_603]);
}
#[test]
fn rook_and_pawn_endgame_depths_one_to_five() {
    verify("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        &[14,191,2812,43_238,674_624]);
}
#[test]
fn tactical_and_promotion_positions() {
    verify("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        &[6,264,9467]);
    verify("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        &[44,1486,62_379]);
}
#[test]
#[ignore = "193 million nodes; run explicitly for the extended perft suite"]
fn kiwipete_depth_five() {
    let mut b=Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1").unwrap();
    assert_eq!(perft(&mut b,5),193_690_690);
}
#[test]
fn make_unmake_castle_ep_promotion() {
    for (fen,moves) in [
        ("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",vec!["e1g1","e8c8"]),
        ("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",vec!["e5d6"]),
        ("4k3/P7/8/8/8/8/8/4K3 w - - 0 1",vec!["a7a8q"]),
    ] {
        let mut b=Board::from_fen(fen).unwrap();
        let original=b.to_fen();let key=b.key;
        let mut played=vec![];
        for text in moves {
            let m=find_uci_move(&mut b,text).expect(text);
            let undo=b.make(m);
            played.push((m,undo));
        }
        for (m,undo) in played.into_iter().rev() {b.unmake(m,undo);}
        assert_eq!(b.to_fen(),original);
        assert_eq!(b.key,key);
    }
}

#[test]
fn illegal_en_passant_unmasking_check_is_rejected() {
    let mut b=Board::from_fen("k3r3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
    assert!(find_uci_move(&mut b,"e5d6").is_none());
}

#[test]
fn castle_through_attacked_square_is_rejected() {
    let mut b=Board::from_fen("r3k2r/8/8/8/8/5r2/8/R3K2R w KQkq - 0 1").unwrap();
    assert!(find_uci_move(&mut b,"e1g1").is_none());
}

#[test]
fn threefold_repetition_is_recorded() {
    let mut b=Board::default();
    let cycle=["g1f3","g8f6","f3g1","f6g8"];
    for text in cycle {let m=find_uci_move(&mut b,text).unwrap();b.make(m);}
    assert!(!b.is_repetition());
    for text in cycle {let m=find_uci_move(&mut b,text).unwrap();b.make(m);}
    assert!(b.is_repetition());
}
