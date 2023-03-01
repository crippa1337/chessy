use crate::{
    constants::{self, MAX_PLY},
    engine::{search::Search, tt::TT},
};
use cozy_chess::{Board, Color, Move, Piece, Square};

#[derive(Debug, PartialEq)]
pub enum SearchType {
    Time(u64),
    Depth(u8),
    Infinite,
}

pub fn main_loop() {
    let mut board = Board::default();
    let mut tt_size = 32;
    let mut tt = TT::new(tt_size);
    let mut search = Search::new(tt);
    let mut uci_set = false;
    let mut board_set = false;

    'input: loop {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        line = line.trim().to_string();
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }

        if !uci_set {
            match words[0] {
                "uci" => {
                    id();
                    options();
                    println!("uciok");
                    uci_set = true;
                    continue;
                }
                "quit" => {
                    break;
                }
                _ => (),
            }
        } else {
            'main: loop {
                match words[0] {
                    "uci" => {
                        id();
                        options();
                        println!("uciok");
                        break 'main;
                    }
                    "isready" => {
                        println!("readyok");
                        break 'main;
                    }
                    "ucinewgame" => {
                        board = Board::startpos();
                        tt = TT::new(tt_size);
                        search = Search::new(tt);
                        board_set = true;
                        break 'main;
                    }
                    "setoption" => {
                        if words[1] == "name" && words[2] == "Hash" && words[3] == "value" {
                            match words[4].parse::<u32>() {
                                Ok(s) => {
                                    // Don't allow hash bigger than max
                                    if s > 1024 {
                                        break 'main;
                                    }
                                    tt_size = s;
                                    tt = TT::new(tt_size);
                                    search = Search::new(tt);
                                }
                                Err(_) => (),
                            }
                        }
                        break 'main;
                    }
                    "position" => {
                        if words[1] == "startpos" {
                            board = Board::startpos();
                            board_set = true;
                            search.game_history = vec![board.hash()]
                        } else if words[1] == "fen" {
                            // Put together the split fen string
                            let mut fen = String::new();
                            for i in 2..words.len() {
                                if words[i] == "moves" {
                                    break;
                                }
                                fen.push_str(words[i]);
                                fen.push(' ');
                            }
                            match Board::from_fen(fen.trim(), false) {
                                Ok(b) => {
                                    board = b;
                                    board_set = true;
                                }
                                Err(_) => (),
                            }
                        }

                        if words.iter().any(|&x| x == "moves") && board_set {
                            for i in
                                words.iter().position(|&x| x == "moves").unwrap() + 1..words.len()
                            {
                                let mut mv: Move = words[i].parse().unwrap();
                                mv = check_castling_move(&board, mv);
                                board.play(mv);
                                search.game_history.push(board.hash());
                            }
                        }
                        break 'main;
                    }
                    "go" => {
                        if board_set {
                            // Static depth search
                            if words.iter().any(|&x| x == "depth") {
                                match words[words.iter().position(|&x| x == "depth").unwrap() + 1]
                                    .parse::<u8>()
                                {
                                    Ok(d) => {
                                        go(&board, SearchType::Depth(d), &mut search);
                                    }
                                    Err(_) => (),
                                }
                            // Infinite search
                            } else if words.iter().any(|&x| x == "infinite") {
                                go(&board, SearchType::Infinite, &mut search);
                            // Static time search
                            } else if words.iter().any(|&x| x == "movetime") {
                                match words
                                    [words.iter().position(|&x| x == "movetime").unwrap() + 1]
                                    .parse::<u64>()
                                {
                                    Ok(d) => {
                                        go(&board, SearchType::Time(d), &mut search);
                                    }
                                    Err(_) => (),
                                }
                            // Time search
                            } else if words.iter().any(|&x| x == "wtime" || x == "btime") {
                                if board.side_to_move() == Color::White {
                                    match words
                                        [words.iter().position(|&x| x == "wtime").unwrap() + 1]
                                        .parse::<u64>()
                                    {
                                        Ok(t) => {
                                            // Increment
                                            let inc: Option<u64> =
                                                if words.iter().any(|&x| x == "winc") {
                                                    match words[words
                                                        .iter()
                                                        .position(|&x| x == "winc")
                                                        .unwrap()
                                                        + 1]
                                                    .parse::<u64>()
                                                    {
                                                        Ok(i) => Some(i),
                                                        Err(_) => None,
                                                    }
                                                } else {
                                                    None
                                                };
                                            let mtg = if words.iter().any(|&x| x == "movestogo") {
                                                match words[words
                                                    .iter()
                                                    .position(|&x| x == "movestogo")
                                                    .unwrap()
                                                    + 1]
                                                .parse::<u8>()
                                                {
                                                    Ok(m) => Some(m),
                                                    Err(_) => None,
                                                }
                                            } else {
                                                None
                                            };

                                            go(
                                                &board,
                                                SearchType::Time(time_for_move(t, inc, mtg)),
                                                &mut search,
                                            );
                                        }
                                        Err(_) => (),
                                    }
                                } else {
                                    match words
                                        [words.iter().position(|&x| x == "btime").unwrap() + 1]
                                        .parse::<u64>()
                                    {
                                        Ok(t) => {
                                            // Increment
                                            let inc: Option<u64> =
                                                if words.iter().any(|&x| x == "binc") {
                                                    match words[words
                                                        .iter()
                                                        .position(|&x| x == "binc")
                                                        .unwrap()
                                                        + 1]
                                                    .parse::<u64>()
                                                    {
                                                        Ok(i) => Some(i),
                                                        Err(_) => None,
                                                    }
                                                } else {
                                                    None
                                                };

                                            let mtg = if words.iter().any(|&x| x == "movestogo") {
                                                match words[words
                                                    .iter()
                                                    .position(|&x| x == "movestogo")
                                                    .unwrap()
                                                    + 1]
                                                .parse::<u8>()
                                                {
                                                    Ok(m) => Some(m),
                                                    Err(_) => None,
                                                }
                                            } else {
                                                None
                                            };

                                            go(
                                                &board,
                                                SearchType::Time(time_for_move(t, inc, mtg)),
                                                &mut search,
                                            );
                                        }
                                        Err(_) => (),
                                    }
                                };
                            } else {
                                break 'main;
                            }
                        }
                        break 'main;
                    }
                    "quit" => {
                        break 'input;
                    }
                    _ => {
                        break 'main;
                    }
                }
            }
        }
    }
}

fn id() {
    println!("id name Svart {}", env!("CARGO_PKG_VERSION"));
    println!("id author crippa");
}

fn options() {
    println!("option name Hash type spin default 32 min 1 max 1024");
}

fn check_castling_move(board: &Board, mut mv: Move) -> Move {
    if board.piece_on(mv.from) == Some(Piece::King) {
        mv.to = match (mv.from, mv.to) {
            (Square::E1, Square::G1) => Square::H1,
            (Square::E8, Square::G8) => Square::H8,
            (Square::E1, Square::C1) => Square::A1,
            (Square::E8, Square::C8) => Square::A8,
            _ => mv.to,
        };
    }
    mv
}

pub fn reverse_castling_move(board: &Board, mut mv: Move) -> Move {
    if board.piece_on(mv.from) == Some(Piece::King) {
        mv.to = match (mv.from, mv.to) {
            (Square::E1, Square::H1) => Square::G1,
            (Square::E8, Square::H8) => Square::G8,
            (Square::E1, Square::A1) => Square::C1,
            (Square::E8, Square::A8) => Square::C8,
            _ => mv.to,
        };
    }
    mv
}

fn go(board: &Board, st: SearchType, search: &mut Search) {
    search.iterative_deepening(board, st);
    reset_search(search);
}

fn time_for_move(time: u64, increment: Option<u64>, moves_to_go: Option<u8>) -> u64 {
    // Account for overhead
    let time = time - constants::TIME_OVERHEAD;

    if let Some(n) = moves_to_go {
        time / n.max(1) as u64
    } else if let Some(n) = increment {
        (time / 20) + (n / 2)
    } else {
        time / 20
    }
}

fn reset_search(search: &mut Search) {
    search.stop = false;
    search.search_type = SearchType::Depth(0);
    search.timer = None;
    search.goal_time = None;
    search.nodes = 0;
    search.seldepth = 0;
    search.killers = [[None; 2]; MAX_PLY as usize];
    search.history.age_table();
    search.tt.age();
}
