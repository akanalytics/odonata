use crate::attacks::{BitboardAttacks, ClassicalBitboard};
use crate::bitboard::{Bitboard, Dir};
use crate::board::{Board, CastlingRights, Color, Piece};
use Vec;

#[derive(Debug)]
pub enum Move {
    Promo { dest: Bitboard, promo: Piece },
    PromoCapture { dest: Bitboard, src: Bitboard, promo: Piece, capture: Piece },
    EnPassant { dest: Bitboard, capture: Piece },
    PawnCapture { dest: Bitboard, src: Bitboard, capture: Piece },
    Push { dest: Bitboard },
    Castle { dest: Bitboard },
    Quiet { dest: Bitboard },
    Null(),
    Drop { dest: Bitboard, piece: Piece },
}

pub struct MoveGen {
    attack_gen: ClassicalBitboard,
}

impl MoveGen {
    // Vec::with_capacity(100).
    fn pseudo_legal_moves(&self, board: &Board, moves: &mut Vec<Move>) {
        let color = Color::WHITE;
        let oppo = Bitboard::EMPTY; //self.board.w.bits
        let prop = Bitboard::EMPTY;
        let occupied = oppo | prop;

        let pawns = Bitboard::EMPTY;

        // non-promoted single-push pawns
        let pawn_push = self.attack_gen.pawn_pushes(occupied, pawns, &color);
        moves.push(Move::Push { dest: pawn_push & !Bitboard::PROMO_RANKS });
        // pawn promos - only makes sense for single push
        for dest in (pawn_push & Bitboard::PROMO_RANKS).iter() {
            // try and pre-sort promos by likely usefulness
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                moves.push(Move::Promo { dest, promo });
            }
        }
        // pawn_captures
        let (pawn_captures_e, pawn_captures_w) = self.attack_gen.pawn_attacks(pawns, &color);
        for dest in (pawn_captures_e & !Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_capture_east.opposite());
            let capture = board.piece_at(dest);
            moves.push(Move::PawnCapture { dest, src, capture });
        }
        for dest in (pawn_captures_w & !Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_capture_west.opposite());
            let capture = board.piece_at(dest);
            moves.push(Move::PawnCapture { dest, src, capture });
        }

        // pawn capture-promos
        for dest in (pawn_captures_e & Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_capture_east.opposite());
            let capture = board.piece_at(dest);
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                moves.push(Move::PromoCapture { dest, src, promo, capture });
            }
        }
        for dest in (pawn_captures_w & Bitboard::PROMO_RANKS).iter() {
            let src = dest.shift(&color.pawn_capture_west.opposite());
            for &promo in &[Piece::Queen, Piece::Knight, Piece::Rook, Piece::Bishop] {
                let capture = board.piece_at(dest);
                moves.push(Move::PromoCapture { dest, src, promo, capture });
            }
        }
    }
    // # pawn_captures e/p - cant be promos
    // if b.en_passant:
    //     for is_east in [True, False]:
    //         en_passant_bb = b.en_passant.bit() if b.en_passant else 0
    //         pawn_captures_ep = self.attack_gen.by_pawns_en_passant(pawns, occupied, oppo, color, en_passant_bb, is_east)
    //         for dst in  bit_iterator(pawn_captures_ep):
    //             delta = 1 if is_east else -1
    //             self.add_move(dst - dir - delta, dst, tags='E/P', promo='', piece='p')
    // assert Clock.capture_as( "MoveGenBB::moves for en passant")

    // # other pieces, we done pawns already in the above
    // for piece in 'nbrqk':
    //     # loop through *our* pieces
    //     attackers = b.pieces(piece).bits & prop
    //     for src in  bit_iterator(attackers):
    //         attacks = self.attack_gen.piece_attacks(piece, empty, Square.of(src).bit())

    //         # remove our own pieces from the officer attacks
    //         attacks &= (Bitwise.mask ^ prop)

    //         for dst in  bit_iterator(attacks):
    //             self.add_move(src, dst, tags='', promo='', piece=piece)
    // assert Clock.capture_as( "MoveGenBB::moves for sliders and steppers")

    // q = Pieces.to_colour('Q', b.turn); queens_side = q in b.castling_rights
    // k = Pieces.to_colour('k', b.turn); kings_side = k in b.castling_rights
    // if q or k:
    //     king = b.pieces('k').bits & prop
    //     opponents = [ b.pawns.bits & oppo, b.knights.bits & oppo, b.bishops.bits & oppo, b.rooks.bits & oppo, b.queens.bits & oppo, b.kings.bits & oppo]
    //     castling_moves = self.attack_gen.king_castling(king, occupied, opponents, kings_side, queens_side, b.turn)
    //     for dst in  bit_iterator(castling_moves):
    //         sq = Bitwise.bit_scan_forward(king)
    //         self.add_move(sq, dst, tags='castling', promo='', piece='k')
    //     assert Clock.capture_as( "MoveGenBB::moves for castling")

    // Clock.capture_as( "MoveGenBB::all")
    // return self.moves

    // castling
    // check castling rights (cheap)
    // check king not in check
    // side = +/-2
    // check king+1 and king+2 for being clear on kings side
    // check king-1, king-2, king-3 clear on queens
    // check that king +/- 1 and king +/- 2 isnt in check
    // addMove King +/- 2, add rook -2/+3
    // castling rights

    fn castling_moves(
        &self,
        king: Bitboard,
        occ: Bitboard,
        board: &Board,
        c: &Color,
        rights: &CastlingRights,
    ) -> Bitboard {
        let mut castlings = Bitboard::EMPTY;
        if rights.intersects(c.castle_rights_king) && !c.kingside_castle_sqs.intersects(occ) {
            let king_moves = king | c.kingside_castle_sqs;
            if self.attacked_by(king_moves, occ, board, c).is_empty() {
                castlings = king.shift(&Dir::E).shift(&Dir::E);
            }
        }
        if rights.intersects(c.castle_rights_queen) && !c.queenside_castle_sqs.intersects(occ) {
            let king_moves = king | c.queenside_castle_sqs;
            if self.attacked_by(king_moves, occ, board, &c).is_empty() {
                castlings = castlings | king.shift(&Dir::W).shift(&Dir::W);
            }
        }
        castlings
    }

    fn attacked_by(&self, target: Bitboard, occ: Bitboard, board: &Board, c: &Color) -> Bitboard {
        let pawns = board.pawns() & board.them();
        let knights = board.knights() & board.them();
        let bishops = board.bishops() & board.them();
        let rooks = board.rooks() & board.them();
        let queens = board.queens() & board.them();
        let kings = board.kings() & board.them();

        let (east, west) = self.attack_gen.pawn_attacks(target, c);
        let mut attackers = (east | west) & pawns;

        let sq = target.first_square();

        attackers |= self.attack_gen.knight_attacks(sq) & knights
            | self.attack_gen.king_attacks(sq) & kings
            | self.attack_gen.bishop_attacks(occ, sq) & (bishops | queens)
            | self.attack_gen.rook_attacks(occ, sq) & (rooks | queens);
        // TODO: en passant!!
        attackers
    }

    // queens = bitboards[Pieces.QUEEN]
    // bishops = bitboards[Pieces.BISHOP]
    // rooks = bitboards[Pieces.ROOK]
    // knights = bitboards[Pieces.KNIGHT]
    // pawns = bitboards[Pieces.PAWN]
    // kings = bitboards[Pieces.KING]

    // attacks_from_officers = (bishop_attacks(bishops | queens, empty) |
    //     rook_attacks(rooks | queens, empty) |
    //     knight_attacks(knights) |
    //     king_attacks(kings))

    // moving_side = Side.opposing_side(target_color)
    // attacks_from_pawns = self.pawn_attacks(pawns, moving_side)
    // return (attacks_from_officers | attacks_from_pawns) & target
}
