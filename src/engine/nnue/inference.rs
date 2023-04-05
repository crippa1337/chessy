// Svart uses a 768->256x2->1 perspective NNUE, largely inspired by Viridithas and Carp.
// A huge thanks to Cosmo and Dede for their help with the implementation.
//
// I hope to further improve the network as well as make the code more original in the future.
use crate::definitions::MAX_PLY;
use cozy_chess::{Board, Color, Piece, Square};

const FEATURES: usize = 768;
const HIDDEN: usize = 256;

// clipped relu bounds
const CR_MIN: i16 = 0;
const CR_MAX: i16 = 255;

// quantization
const QAB: i32 = 255 * 64;
const SCALE: i32 = 400;

pub const ACTIVATE: bool = true;
pub const DEACTIVATE: bool = false;

struct Parameters {
    feature_weights: [i16; FEATURES * HIDDEN],
    feature_bias: [i16; HIDDEN],
    output_weights: [i16; HIDDEN * 2], // perspective aware
    output_bias: i16,
}

// the model is read from binary files at compile time
static MODEL: Parameters = Parameters {
    feature_weights: unsafe { std::mem::transmute(*include_bytes!("net/feature_weights.bin")) },
    feature_bias: unsafe { std::mem::transmute(*include_bytes!("net/feature_bias.bin")) },
    output_weights: unsafe { std::mem::transmute(*include_bytes!("net/output_weights.bin")) },
    output_bias: unsafe { std::mem::transmute(*include_bytes!("net/output_bias.bin")) },
};

pub struct NNUEState {
    pub accumulators: [Accumulator; MAX_PLY as usize],
    pub current_acc: usize,
}

// The accumulator represents the
// hidden layer from both perspectives
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Accumulator {
    white: [i16; HIDDEN],
    black: [i16; HIDDEN],
}

impl Default for Accumulator {
    fn default() -> Self {
        Self { white: MODEL.feature_bias, black: MODEL.feature_bias }
    }
}

impl Accumulator {
    // efficiently update the change of a feature
    fn efficiently_update<const ACTIVATE: bool>(&mut self, idx: (usize, usize)) {
        fn update_perspective<const ACTIVATE: bool>(acc: &mut [i16; HIDDEN], idx: usize) {
            // we iterate over the weights corresponding to the feature that has been changed
            // and then update the activations in the hidden layer accordingly
            let feature_weights = acc
                .iter_mut()
                // the column of the weight matrix corresponding to the index of the feature
                .zip(&MODEL.feature_weights[idx..idx + HIDDEN]);

            for (activation, &weight) in feature_weights {
                if ACTIVATE {
                    *activation += weight;
                } else {
                    *activation -= weight;
                }
            }
        }

        update_perspective::<ACTIVATE>(&mut self.white, idx.0);
        update_perspective::<ACTIVATE>(&mut self.black, idx.1);
    }
}

impl NNUEState {
    // Referencing Viridithas' implementation:
    //
    // The NNUEState is too large to be allocated on the stack at the time of writing, so we have to box it.
    // This is done by allocating the memory manually and then constructing the object in place.
    // Why not just box normally? Because rustc in debug mode will first allocate on the stack
    // before moving it to the heap when boxxing, which would blow the stack.
    pub fn from_board(board: &Board) -> Box<Self> {
        let mut boxed: Box<NNUEState> = unsafe {
            let layout = std::alloc::Layout::new::<Self>();
            let ptr = std::alloc::alloc_zeroed(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            Box::from_raw(ptr.cast())
        };

        // initialize the first state
        boxed.accumulators[0] = Accumulator::default();
        for sq in board.occupied() {
            let piece = board.piece_on(sq).unwrap();
            let color = board.color_on(sq).unwrap();
            let idx = weight_column_index(sq, piece, color);

            boxed.accumulators[0].efficiently_update::<ACTIVATE>(idx);
        }

        boxed
    }

    pub fn refresh(&mut self, board: &Board) {
        // reset the accumulator stack
        self.current_acc = 0;
        self.accumulators[self.current_acc] = Accumulator::default();

        // update the first accumulator
        for sq in board.occupied() {
            let piece = board.piece_on(sq).unwrap();
            let color = board.color_on(sq).unwrap();
            let idx = weight_column_index(sq, piece, color);

            self.accumulators[self.current_acc].efficiently_update::<ACTIVATE>(idx);
        }
    }

    /// Copy and push the current accumulator to the "top"
    pub fn push(&mut self) {
        self.accumulators[self.current_acc + 1] = self.accumulators[self.current_acc];
        self.current_acc += 1;
    }

    pub fn pop(&mut self) {
        self.current_acc -= 1;
    }

    pub fn update_feature<const ACTIVATE: bool>(&mut self, sq: Square, piece: Piece, color: Color) {
        let idx = weight_column_index(sq, piece, color);

        self.accumulators[self.current_acc].efficiently_update::<ACTIVATE>(idx);
    }

    pub fn evaluate(&self, stm: Color) -> i32 {
        let acc = &self.accumulators[self.current_acc];

        let (us, them) = match stm {
            Color::White => (acc.white.iter(), acc.black.iter()),
            Color::Black => (acc.black.iter(), acc.white.iter()),
        };

        // Add on the bias
        let mut output = MODEL.output_bias as i32;

        // Add on the activations from one perspective with clipped ReLU
        for (&value, &weight) in us.zip(&MODEL.output_weights[..HIDDEN]) {
            output += (value.clamp(CR_MIN, CR_MAX) as i32) * (weight as i32);
        }

        // ... other perspective
        for (&value, &weight) in them.zip(&MODEL.output_weights[HIDDEN..]) {
            output += (value.clamp(CR_MIN, CR_MAX) as i32) * (weight as i32);
        }

        // Quantization
        output * SCALE / QAB
    }
}

// Returns white's and black's feature weight index respectively
// i.e where the feature's weight column is in the weight matrix.
#[must_use]
fn weight_column_index(sq: Square, piece: Piece, color: Color) -> (usize, usize) {
    // The jump from one perspective to the other
    const COLOR_STRIDE: usize = 64 * 6;
    // The jump from one piece type to the next
    const PIECE_STRIDE: usize = 64;
    let p = match piece {
        Piece::Pawn => 0,
        Piece::Knight => 1,
        Piece::Bishop => 2,
        Piece::Rook => 3,
        Piece::Queen => 4,
        Piece::King => 5,
    };

    let c = color as usize;

    let white_idx = c * COLOR_STRIDE + p * PIECE_STRIDE + sq as usize;
    let black_idx = (1 ^ c) * COLOR_STRIDE + p * PIECE_STRIDE + sq.flip_rank() as usize;

    (white_idx * HIDDEN, black_idx * HIDDEN)
}

#[cfg(test)]
mod tests {
    use crate::engine::{movegen, position::play_move, search::Search, tt::TT};

    use super::*;

    #[test]
    fn nnue_indexing() {
        let idx1 = weight_column_index(Square::A8, Piece::Pawn, Color::White);
        let idx2 = weight_column_index(Square::H1, Piece::Pawn, Color::White);
        let idx3 = weight_column_index(Square::A1, Piece::Pawn, Color::Black);
        let idx4 = weight_column_index(Square::E1, Piece::King, Color::White);

        assert_eq!(idx1, (14336, 98304));
        assert_eq!(idx2, (1792, 114432));
        assert_eq!(idx3, (98304, 14336));
        assert_eq!(idx4, (82944, 195584));
    }

    #[test]
    fn nnue_update_feature() {
        let board: Board = Board::default();
        let mut state = NNUEState::from_board(&board);

        let old_acc = state.accumulators[0];

        state.update_feature::<ACTIVATE>(Square::A3, Piece::Pawn, Color::White);
        state.update_feature::<DEACTIVATE>(Square::A3, Piece::Pawn, Color::White);

        assert_eq!(old_acc, state.accumulators[0]);
    }

    #[test]
    fn nnue_moves() {
        let board = Board::default();
        let tt = TT::new(16);
        let mut search = Search::new(tt);
        let moves = movegen::all_moves(&search, &board, None, 0);
        let initial_white = search.nnue.accumulators[0].white;
        let initial_black = search.nnue.accumulators[0].black;
        for mv in moves {
            let mv = mv.mv;
            let mut new_b = board.clone();
            play_move(&mut new_b, &mut search.nnue, mv);
            assert_ne!(initial_white, search.nnue.accumulators[1].white);
            assert_ne!(initial_black, search.nnue.accumulators[1].black);
            search.nnue.pop();
            assert_eq!(initial_white, search.nnue.accumulators[0].white);
            assert_eq!(initial_black, search.nnue.accumulators[0].black);
        }
    }

    #[test]
    fn nnue_incremental() {
        let fens: [&str; 13] = [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "4r1k1/4r1p1/8/p2R1P1K/5P1P/1QP3q1/1P6/3R4 b - - 0 1",
            "1n2k3/r3r1bn/pp1p4/1P1q1p1p/3P4/P3P1RP/1BQN1PR1/1K6 w - - 6 28",
            "8/3r1b2/3r1Pk1/1N6/5ppP/1q2P1Q1/7K/4RB2 w - - 0 66",
            "rnbqkbnr/1pp1ppp1/p7/2PpP2p/8/8/PP1P1PPP/RNBQKBNR w KQkq d6 0 5",
            "rnbqkbnr/1pp1p3/p4pp1/2PpP2p/8/3B1N2/PP1P1PPP/RNBQK2R w KQkq - 0 7",
            "rnbqk2r/1pp1p1P1/p4np1/2Pp3p/8/3B1N2/PP1P1PPP/RNBQK2R w KQkq - 1 9",
            "rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2",
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        ];

        for fen in fens {
            let mut board = Board::from_fen(fen, false).unwrap();
            let tt = TT::new(16);
            let mut search = Search::new(tt);
            search.nnue.refresh(&board);
            let moves = movegen::all_moves(&search, &board, None, 0);

            for mv in moves {
                let mut board2 = Board::from_fen(fen, false).unwrap();

                board2.play_unchecked(mv.mv);
                play_move(&mut board, &mut search.nnue, mv.mv);

                let state2 = NNUEState::from_board(&board2);
                assert_eq!(search.nnue.accumulators[1], state2.accumulators[0]);
                assert_ne!(search.nnue.accumulators[0], state2.accumulators[0]);

                search.nnue.pop();
                board = Board::from_fen(fen, false).unwrap();
            }
        }
    }
}
