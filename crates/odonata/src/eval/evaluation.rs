use crate::{
    bits::{bitboard::Dir, precalc::Pawns, CastlingRights, Square},
    boards::Board,
    eg::LikelyOutcome,
    eg::EndGame,
    infra::metric::Metrics,
    search::node::Timing,
    Color, Piece, PreCalc,
};

use super::{scoring::Scorer, Feature};
use crate::prelude::*;

pub struct Eval;

impl Eval {
    pub fn eval<T>(&mut self, b: &Board, sc: &mut impl Scorer<T>) {
        use crate::Color::*;
        let t = Metrics::timing_start();
        self.material(b, sc);
        if !self.endgame(b, sc) {
            // let pawn_cache = UnsharedTable::<PawnStructure>::default();
            // self.set_pawn_structure(&pawn_cache);
            let pawn_structure = Pawns::new(b.pawns() & b.white(), b.pawns() & b.black());
            self.pawns_both(b, &pawn_structure, sc);
            self.position(b, sc);
            self.pst(b, sc);
            self.other(b, sc);
            self.king_safety(White, b, sc);
            self.king_safety(Black, b, sc);
            self.mobility(White, b, sc);
            self.mobility(Black, b, sc);
        }
        // scorer.set_phase(b.phase(ph));
        // scorer.interpolate_and_scale("interpolate");
        Metrics::profile(t, Timing::TimingEval);
    }

    #[inline(never)]
    fn material<T>(&mut self, b: &Board, sc: &mut impl Scorer<T>) {
        use crate::{Color::*, Piece::*};
        use Feature::*;
        let m = b.material();

        sc.accum(
            Feature::MaterialPawn,
            m.count(White, Pawn),
            m.count(Black, Pawn),
        );
        sc.accum(
            Feature::MaterialKnight,
            m.count(White, Knight),
            m.count(Black, Knight),
        );
        sc.accum(
            Feature::MaterialBishop,
            m.count(White, Bishop),
            m.count(Black, Bishop),
        );
        sc.accum(
            Feature::MaterialRook,
            m.count(White, Rook),
            m.count(Black, Rook),
        );
        sc.accum(
            Feature::MaterialQueen,
            m.count(White, Queen),
            m.count(Black, Queen),
        );

        sc.accum(
            BishopPair,
            (m.count(White, Bishop) >= 2) as i32,
            (m.count(Black, Bishop) >= 2) as i32,
        );
        sc.accum(
            RookPair,
            (m.count(White, Rook) >= 2) as i32,
            (m.count(Black, Rook) >= 2) as i32,
        );
    }

    fn other<T>(&mut self, b: &Board, sc: &mut impl Scorer<T>) {
        use crate::Color::*;
        use Feature::*;
        sc.accum(
            TempoBonus,
            (b.color_us() == White) as i32,
            (b.color_us() == Black) as i32,
        );
    }

    #[inline(never)]
    fn position<T>(&mut self, b: &Board, sc: &mut impl Scorer<T>) {
        use crate::Color::*;
        use Feature::*;
        // fianchetto (short)
        const W_BISHOP: Bitboard = Bitboard::G2;
        const W_KING: Bitboard = Bitboard::F1.or(Bitboard::G1).or(Bitboard::H1);
        const W_PAWNS: Bitboard = Bitboard::F2
            .or(Bitboard::G3)
            .or(Bitboard::H2)
            .or(Bitboard::H3)
            .or(Bitboard::H4);
        const W_NO_PAWNS: Bitboard = Bitboard::F3;

        const B_BISHOP: Bitboard = W_BISHOP.flip_vertical();
        const B_KING: Bitboard = W_KING.flip_vertical();
        const B_PAWNS: Bitboard = W_PAWNS.flip_vertical();
        const B_NO_PAWNS: Bitboard = W_NO_PAWNS.flip_vertical();

        let fianchetto = |c: Color| {
            let us = b.color(c);
            // let w = b.color(White);
            // let b = b.color(Black);
            let bishop = c.chooser_wb(W_BISHOP, B_BISHOP);
            let pawns = c.chooser_wb(W_PAWNS, B_PAWNS);
            let no_pawns = c.chooser_wb(W_NO_PAWNS, B_NO_PAWNS);
            let king = c.chooser_wb(W_KING, B_KING);

            ((b.bishops() & us).intersects(bishop)
                && (b.pawns() & us & pawns).popcount() >= 3
                && (b.pawns() & us).disjoint(no_pawns)
                && (b.kings() & us).intersects(king)) as i32
        };
        sc.accum(Fianchetto, fianchetto(White), fianchetto(Black));

        // if queen has moved but other pieces havent (FIXME! not quite exactly right (QxQ))
        let queen_early_develop = |c: Color| {
            let us = b.color(c);
            if (us & b.queens() & Bitboard::FILE_D & Bitboard::RANKS_18).is_empty() {
                (us & Bitboard::RANKS_18
                    & ((b.bishops() & (Bitboard::FILE_C.or(Bitboard::FILE_F)))
                        | (b.knights() & (Bitboard::FILE_B.or(Bitboard::FILE_G)))))
                .popcount()
            } else {
                0
            }
        };
        sc.accum(
            QueenEarlyDevelop,
            queen_early_develop(White),
            queen_early_develop(Black),
        );
    }

    #[inline(never)]
    fn endgame<T>(&mut self, b: &Board, sc: &mut impl Scorer<T>) -> bool {
        let endgame = EndGame::from_board(b);
        use Feature::*;
        // match endgame.likely_outcome(b) {
        //     LikelyOutcome::DrawImmediate | LikelyOutcome::Draw => return true,
        //     _ => {},
        // };

        // self.material(scorer, b);

        if let Some(winner) = endgame.try_winner(b) {
            if let Some((metric1, metric2)) = endgame.metrics(winner, b) {
                sc.accum2(winner, WinMetric1, -metric1);
                sc.accum2(winner, WinMetric2, -metric2);

                // win specific scoring, so we award win_bonus as other features will be ignored
                sc.accum2(winner, WinBonus, 1);

                // as we wont be adding structural or mobility things we add another bonus
                sc.accum2(winner, CertainWinBonus, 1);
                return true;
            }
            // award a win bonus even if we dont have win-specific scoring
            sc.accum2(winner, WinBonus, 1);
            return false; // TODO! we have a winner, but no specific win scoring. Do we still use just material
        }
        match endgame.likely_outcome(b) {
            LikelyOutcome::Draw | LikelyOutcome::DrawImmediate => {}
            _ => sc.apply_scaling(1.),
        };
        false
    }

    #[inline(never)]
    fn pst<T>(&mut self, b: &Board, sc: &mut impl Scorer<T>) {
        // if s.csv() {
        for &p in &Piece::ALL {
            let w = (b.pieces(p) & b.white()).flip_vertical();
            let b = b.pieces(p) & b.black();

            for sq in (w | b).squares() {
                // FIXME performance of from_index
                let f = Feature::from_index(Feature::index_pst(Feature::PstP_A1, p, sq));
                sc.accum(f, sq.is_in(w) as i32, sq.is_in(b) as i32);
            }
        }
    }
    // #[inline]
    // fn net(
    // &mut self,
    // f: Feature,
    // w: Bitboard,
    // b: Bitboard,
    // sc: &mut impl Scorer<T>,
    // ) {
    // wt(f, w.popcount(), b.popcount());
    // }
    //
    #[inline(never)]
    fn pawns_both<T>(&mut self, bd: &Board, p: &Pawns, s: &mut impl Scorer<T>) {
        use crate::{eval::evaluation::Feature::*, globals::constants::*};
        const BR_7: Bitboard = RANK_7.flip_vertical();
        const BR_6: Bitboard = RANK_6.flip_vertical();
        const BR_5: Bitboard = RANK_5.flip_vertical();
        const BR_4: Bitboard = RANK_4.flip_vertical();
        const R_67: Bitboard = RANK_6.or(RANK_7);
        const BR_67: Bitboard = R_67.flip_vertical();
        const R_345: Bitboard = RANK_3.or(RANK_4).or(RANK_5);
        const BR_345: Bitboard = R_345.flip_vertical();
        const R_2345: Bitboard = RANK_2.or(R_345);
        const BR_2345: Bitboard = R_2345.flip_vertical();

        #[inline(always)]
        fn net<T>(s: &mut impl Scorer<T>, attr: Feature, w: Bitboard, b: Bitboard) {
            s.accum(attr, w.popcount(), b.popcount());
        }

        let w = bd.white(); // white pieces (not just pawns)
        let b = bd.black();
        // Pawns::new(bd.pawns() & w, bd.pawns() & b);

        let is_far_pawns = (bd.pawns() & Bitboard::FILE_A.or(Bitboard::FILE_B)).any()
            && (bd.pawns() & Bitboard::FILE_G.or(Bitboard::FILE_H)).any()
            && bd.rooks_or_queens().is_empty();
        let wbishops = (bd.bishops() & w).popcount();
        let bbishops = (bd.bishops() & b).popcount();
        s.accum(
            BishopFarPawns,
            is_far_pawns as i32 * wbishops,
            is_far_pawns as i32 * bbishops,
        );

        let mut closedness = p.rammed.popcount() / 2;

        net(s, Closedness, p.rammed, p.rammed);

        // net(
        //     s,
        //     PotentialOutpost,
        //     p.white_controlled
        //         & p.white_outposts
        //         & !bd.occupied()
        //         & (Bitboard::RANK_4 | Bitboard::RANK_5 | Bitboard::RANK_6),
        //         // & !Bitboard::EDGE,
        //     p.black_controlled
        //         & p.black_outposts
        //         & !bd.occupied()
        //         & (Bitboard::RANK_5 | Bitboard::RANK_4 | Bitboard::RANK_3),
        //         // & !Bitboard::EDGE,
        // );

        // try and ensure closedness is symmetric
        let centerish_rammed_pawns =
            p.rammed & w & (Bitboard::CENTER_16_SQ | Bitboard::CENTER_16_SQ.shift(Dir::S))
                | p.rammed & b & (Bitboard::CENTER_16_SQ | Bitboard::CENTER_16_SQ.shift(Dir::N));
        closedness += centerish_rammed_pawns.popcount();
        closedness = closedness * closedness;

        // s.accum(c, Closedness, closedness);
        s.accum(
            KnightClosedness,
            (bd.knights() & w).popcount() * closedness,
            (bd.knights() & b).popcount() * closedness,
        );
        s.accum(
            BishopClosedness,
            (bd.bishops() & w).popcount() * closedness,
            (bd.bishops() & b).popcount() * closedness,
        );
        s.accum(
            RookClosedness,
            (bd.rooks() & w).popcount() * closedness,
            (bd.rooks() & b).popcount() * closedness,
        );

        let bishop_color_rammed_pawns = p.bishop_colored_rammed(bd);
        net(
            s,
            BishopColorRammedPawns,
            bishop_color_rammed_pawns & w,
            bishop_color_rammed_pawns & b,
        );
        // double count those pawns near the center
        net(
            s,
            BishopColorRammedPawns,
            bishop_color_rammed_pawns & w, // & Bitboard::CENTER_16_SQ.shift(Dir::S),
            bishop_color_rammed_pawns & b, // & Bitboard::CENTER_16_SQ.shift(Dir::N),
        );

        let connected_bishop_w =
            bd.bishops() & w & (p.white_single_attacks | p.white_double_attacks);
        // let connected_bishop_w = connected_bishop_w - (bd.bishops() & b).squares_of_matching_color();
        // let connected_bishop_w = connected_bishop_w.iff((bd.knights() & b).is_empty());

        let connected_bishop_b =
            bd.bishops() & b & (p.black_single_attacks | p.black_double_attacks);
        // let connected_bishop_b = connected_bishop_b - (bd.bishops() & w).squares_of_matching_color();
        // let connected_bishop_b = connected_bishop_b.iff((bd.knights() & w).is_empty());

        net(s, BishopConnected, connected_bishop_w, connected_bishop_b);

        // doubled not isolated
        net(
            s,
            PawnDoubled,
            p.doubled & !p.isolated & w,
            p.doubled & !p.isolated & b,
        );
        net(
            s,
            PawnIsolatedDoubled,
            p.doubled & p.isolated & w,
            p.doubled & p.isolated & b,
        );
        // s.accum(
        //     c,
        //     PawnDirectlyDoubled,
        //     _pawn_directly_doubled,
        // );
        net(s, PawnWeak, p.weak & w, p.weak & b);
        net(s, PawnIsolated, p.isolated & w, p.isolated & b);

        // TODO: PawnIsolatedHalfOpen
        // net(
        //     s,
        //     PawnIsolatedHalfOpen,
        //     (p.isolated & w & p.half_open).iff(bd.rooks().any()),
        //     (p.isolated & b & p.half_open).iff(bd.rooks().any()),
        // );

        // TODO: SemiIsolated
        // let _semi_isolated = if !p.is_in(isolated_pawns_bb)
        //     && (((bbd.neighbouring_files(p) & bbd.within_chebyshev_distance_inclusive(p, 2))
        //         - p.as_bb()
        //         - p.file())
        //         & b.pawns()
        //         & us)
        //         .is_empty()

        net(s, SemiIsolated, Bitboard::EMPTY, Bitboard::EMPTY);
        let dn = p.distant_neighbours;
        net(s, PawnDistantNeighboursR7, dn & RANK_7 & w, dn & BR_7 & b);
        net(s, PawnDistantNeighboursR6, dn & RANK_6 & w, dn & BR_6 & b);
        net(s, PawnDistantNeighboursR5, dn & RANK_5 & w, dn & BR_5 & b);
        net(s, PawnPassed, p.passed & w, p.passed & b);
        net(s, PawnPassedR7, p.passed & RANK_7 & w, p.passed & BR_7 & b);
        net(s, PawnPassedR6, p.passed & RANK_6 & w, p.passed & BR_6 & b);
        net(s, PawnPassedR5, p.passed & RANK_5 & w, p.passed & BR_5 & b);
        net(s, PawnPassedR4, p.passed & RANK_4 & w, p.passed & BR_4 & b);
        net(
            s,
            PassersOnRim,
            p.passed & Bitboard::RIM & w,
            p.passed & Bitboard::RIM & b,
        );
        net(
            s,
            CandidatePassedPawn,
            p.candidate_passed & w & Bitboard::home_half(Color::Black),
            p.candidate_passed & b & Bitboard::home_half(Color::White),
        );
        let blockaded_opponent = p.blockaded_opponent(&bd);
        let blockaded_self = p.blockaded_self(&bd);
        let blockaded_any = blockaded_self | blockaded_opponent;
        // net(s, BlockadedOpponent, blockaded_opponent & w, blockaded_opponent & b);
        // net(s, BlockadedSelf, blockaded_self & w, blockaded_self & b);
        net(s, BlockadedAny, blockaded_any & w, blockaded_any & b);
        net(
            s,
            BlockadedPassers,
            blockaded_opponent & p.passed & w,
            blockaded_opponent & p.passed & b,
        );

        // TODO:PawnDoubleAttacks
        // pawn forks or double attacks
        // let bnp = b - bd.pawns();
        // let wnp = w - bd.pawns();
        // let wf = (p.white.shift(Dir::N) & !bd.occupied() & bnp.shift(Dir::SW) & bnp.shift(Dir::SE)).iff(bd.color_us() == White).popcount();
        // let bf = (p.black.shift(Dir::S) & !bd.occupied() & wnp.shift(Dir::NW) & wnp.shift(Dir::NE)).iff(bd.color_us() == Black).popcount();

        // let wf = ((p.white_single_attacks | p.white_double_attacks) & (b - bd.pawns())).two_or_more() as i32;
        // let bf = ((p.black_single_attacks | p.black_double_attacks) & (w - bd.pawns())).two_or_more() as i32;
        // s.accumulate(PawnDoubleAttacks, wf, bf);

        let rbp = p.rooks_behind_passers(&bd);
        net(s, RooksBehindPasser, rbp & w, rbp & b);
        // scorer.accum(c, RammedPawns, rammed_pawns);

        // TODO: Eval of PawnIsolatedDoubled
        net(s, PawnIsolatedDoubled, Bitboard::EMPTY, Bitboard::EMPTY);

        // TODO: Eval of Space
        net(s, Space, Bitboard::EMPTY, Bitboard::EMPTY);
        net(
            s,
            PawnConnectedR67,
            p.connected & R_67 & w,
            p.connected & BR_67 & b,
        );
        net(
            s,
            PawnConnectedR345,
            p.connected & R_345 & w,
            p.connected & BR_345 & b,
        );
        // TODO: PassedConnectedR67 & PassedConnectedR345
        //     if p.is_in(pawn_atts) || p.is_in(pawn_duos)  {
        net(s, PassedConnectedR67, Bitboard::EMPTY, Bitboard::EMPTY);
        net(s, PassedConnectedR345, Bitboard::EMPTY, Bitboard::EMPTY);

        net(s, PawnDuoR67, p.duos & R_67 & w, p.duos & BR_67 & b);
        net(s, PawnDuoR2345, p.duos & R_2345 & w, p.duos & BR_2345 & b);

        // TODO:PassedDuoR67 & PassedDuoR2345
        net(s, PassedDuoR67, Bitboard::EMPTY, Bitboard::EMPTY);
        net(s, PassedDuoR2345, Bitboard::EMPTY, Bitboard::EMPTY);
        net(
            s,
            BackwardHalfOpen,
            p.backward & p.half_open & w,
            p.backward & p.half_open & b,
        );
        net(
            s,
            Backward,
            p.backward & !p.half_open & w,
            p.backward & !p.half_open & b,
        );
    }

    #[inline(never)]
    fn king_safety<T>(&mut self, c: Color, b: &Board, sc: &mut impl Scorer<T>) {
        use Feature::*;
        let us = b.color(c);
        let k = b.kings() & us;
        if k.is_empty() {
            return;
        }
        let them = b.color(c.opposite());
        let p = b.pawns() & us;
        let ksq = k.square();
        let bb = PreCalc::instance();
        let castling_rights = b.castling().contains(CastlingRights::king_side_right(c)) as i32
            + b.castling().contains(CastlingRights::queen_side_right(c)) as i32;

        let uncastled = (bb
            .within_chebyshev_distance_inclusive(Square::E8, 1)
            .or(bb.within_chebyshev_distance_inclusive(Square::E1, 1))
            .contains(k)) as i32;

        // tropism
        //
        let d1 = bb.within_chebyshev_distance_inclusive(ksq, 1);
        let d2 = bb.within_chebyshev_distance_inclusive(ksq, 2);
        let d3 = bb.within_chebyshev_distance_inclusive(ksq, 3);
        let d4 = bb.within_chebyshev_distance_inclusive(ksq, 4);
        let king_tropism_d1 = (d1 & (b.pawns() | b.kings()) & them).popcount()
            + (d1 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d1 & (b.rooks()) & them).popcount() * 2
            + (d1 & (b.queens()) & them).popcount() * 4;

        let king_tropism_d2 = (d2 & (b.pawns() | b.kings()) & them).popcount()
            + (d2 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d2 & (b.rooks()) & them).popcount() * 2
            + (d2 & (b.queens()) & them).popcount() * 4;

        let king_tropism_d3 = (d3 & (b.pawns() | b.kings()) & them).popcount()
            + (d3 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d3 & (b.rooks()) & them).popcount() * 2
            + (d3 & (b.queens()) & them).popcount() * 4;

        let king_tropism_d4 = (d4 & (b.pawns() | b.kings()) & them).popcount()
            + (d3 & (b.knights() | b.bishops()) & them).popcount() * 2
            + (d3 & (b.rooks()) & them).popcount() * 2
            + (d3 & (b.queens()) & them).popcount() * 4;
        sc.accum2(c, TropismD1, king_tropism_d1);
        sc.accum2(c, TropismD2, king_tropism_d2);
        sc.accum2(c, TropismD3, king_tropism_d3);
        sc.accum2(c, TropismD4, king_tropism_d4);

        // pawn shield
        //
        let (adjacent, nearby) = bb.adjacent_and_nearby_pawn_shield(c, ksq);
        let adjacent_shield = (p & adjacent).popcount();
        sc.set_bits(PawnAdjacentShield, adjacent & p);
        let nearby_shield = (p & nearby).popcount();
        sc.set_bits(PawnNearbyShield, nearby & p);
        let isolated_shield = bb.pawn_side_isolated(p) & (adjacent | nearby);
        sc.accum2(c, PawnAdjacentShield, adjacent_shield);
        sc.accum2(c, PawnNearbyShield, nearby_shield);
        sc.accum2(c, PawnShieldFaulty, isolated_shield.popcount());
        let king_safety_bonus = if b.queens().any() {
            adjacent_shield + nearby_shield
        } else {
            0
        };
        sc.accum2(c, KingSafetyBonus, king_safety_bonus);

        // storming pawms - from POV of defending king color c

        // ranks 4-7
        let storm_ranks = Bitboard::home_half(c.opposite())
            .shift(c.backward())
            .shift(c.backward())
            .shift(c.backward());
        let storm_files = ksq.kq_side();
        let storm = storm_ranks & storm_files;
        let storming_pawns = b.pawns() & them & storm;
        let blocked = storming_pawns & (b.pawns() & us).shift(c.forward());
        let unblocked = storming_pawns - blocked;
        sc.set_bits(StormBlocked, blocked);
        sc.set_bits(StormUnblocked, unblocked);
        let blocked_r3 = blocked & c.rank_as_white_num(3);
        let blocked_r4 = blocked & c.rank_as_white_num(4);
        let unblocked_r23 = unblocked & (c.rank_as_white_num(2) | c.rank_as_white_num(3));
        sc.accum2(
            c,
            StormBlocked,
            blocked.popcount() - blocked_r3.popcount() - blocked_r4.popcount(),
        );
        sc.accum2(c, StormBlockedR3, blocked_r3.popcount());
        sc.accum2(c, StormBlockedR4, blocked_r4.popcount());
        sc.accum2(
            c,
            StormUnblocked,
            unblocked.popcount() - unblocked_r23.popcount(),
        );
        sc.accum2(c, StormUnblockedR23, unblocked_r23.popcount());

        let open_files_near_king_bb = d3 & ksq.rank() & bb.open_files(b.pawns());
        let open_files_near_king = (open_files_near_king_bb).popcount();
        let open_files_adjacent_king = (d1 & ksq.rank() & bb.open_files(b.pawns())).popcount();
        let rq_on_open_files_near_king =
            (open_files_near_king_bb.file_flood() & b.rooks_or_queens() & them).popcount();

        let king_trapped_on_back_rank = (b.rooks_or_queens().any()
            && k.intersects(Bitboard::RANKS_18)
            // && Bitboard::RANKS_18 & ksq.rank() & us == k
            && (d1 - Bitboard::RANKS_18 - b.occupied()).is_empty())
            as i32;

        let checkers = b.checkers_of(c).popcount();

        let pieces_near_king = (d2 & b.occupied() & us & (b.queens() | b.knights())).popcount();
        let pinned_near_king = (b.pinned(c) & d1).popcount();
        let pinned_far = (b.pinned(c)).popcount() - pinned_near_king;
        let discovered_checks = (b.discoverer(c) - b.pawns()).popcount();

        let tempo_safety_d12 = if b.color_them() == c {
            king_tropism_d1 + king_tropism_d2 + open_files_adjacent_king + open_files_near_king
        } else {
            0
        };
        sc.accum2(c, OpenFilesNearKing, open_files_near_king);
        sc.accum2(c, OpenFilesAdjacentKing, open_files_adjacent_king);
        sc.accum2(c, TempoSafety, tempo_safety_d12);
        sc.accum2(c, KingTrappedOnBackRank, king_trapped_on_back_rank);
        sc.accum2(c, RqOnOpenFilesNearKing, rq_on_open_files_near_king);

        sc.accum2(c, CastlingRightsBonus, castling_rights);
        sc.accum2(c, Uncastled, uncastled);
        sc.accum2(c, Checkers, checkers);
        sc.accum2(c, PiecesNearKing, pieces_near_king);
        sc.accum2(c, PinnedNearKing, pinned_near_king);
        sc.accum2(c, PinnedFar, pinned_far);
        sc.accum2(c, DiscoveredChecks, discovered_checks);
    }

    #[inline(never)]
    fn mobility<T>(&mut self, c: Color, b: &Board, s: &mut impl Scorer<T>) {
        use Color::*;
        use Feature::*;
        let bb = PreCalc::instance();
        let us = b.color(c);
        let opponent = c.opposite();
        let them = b.color(opponent);
        let occ = them | us;
        let empty = !occ;
        let open_files = bb.open_files(b.pawns());
        let semi_open_files = bb.open_files(b.pawns() & us) - open_files; // free of our pawns
        let their_p = b.pawns() & them;
        let our_p = b.pawns() & us;
        let (pe, pw) = bb.pawn_attacks_ew(their_p, opponent);
        let (ope, opw) = bb.pawn_attacks_ew(our_p, c);
        let pa = pe | pw;
        let our_pa = ope | opw;
        let bi = b.bishops() & them;
        let ni = b.knights() & them;
        let r = b.rooks() & them;
        let _q = b.queens() & them;

        // general
        let mut _partially_trapped_pieces = 0;
        let mut _fully_trapped_pieces = 0;
        let mut attacks_near_king = 0;
        // let mut moves_near_king = 0;
        let mut _move_squares = 0;
        // let mut non_pawn_defended_moves = 0;
        let mut center_attacks = 0;
        let mut all_attacks = Bitboard::empty();
        let mut double_attacks = Bitboard::empty();

        // fewer pawns rook bonus

        // knight
        let mut knight_connected = false;
        let knight_outpost = 0;
        let knight_outpost_rook_safe = 0;
        let mut knight_outpost_pawn_defended = 0;
        let mut knight_forks = 0;
        let mut knight_attacks_center = 0;
        let knight_trapped = 0;

        // bishop
        let mut bishop_outposts = 0;
        let bishop_trapped = 0;

        // rook
        let mut enemy_pawns_on_rook_rank = 0;
        let connected_rooks = false;
        let rook_trapped = 0;

        // queen
        let queen_trapped = 0;
        let queens_on_open_files = (open_files & us & b.queens()).popcount();

        let k = b.kings() & them;

        let (adjacent, nearby) = if k.any() {
            bb.adjacent_and_nearby_pawn_shield(opponent, k.square())
        } else {
            (Bitboard::empty(), Bitboard::empty())
        };
        let pawn_shield = adjacent | nearby;
        // let pawn_shield = bb.within_chebyshev_distance_inclusive(ksq, 1);

        // attacks[c][p]
        // let our_atts = if c == Color::White {
        //     [
        //         bb.all_attacks(Color::White, Piece::Pawn, b.pawns() & b.white(), occ),
        //         bb.all_attacks(Color::White, Piece::Knight, b.knights() & b.white(), occ),
        //         bb.all_attacks(Color::White, Piece::Bishop, b.bishops() & b.white(), occ),
        //         bb.all_attacks(Color::White, Piece::Rook, b.rooks() & b.white(), occ),
        //         bb.all_attacks(Color::White, Piece::Queen, b.queens() & b.white(), occ),
        //         bb.all_attacks(Color::White, Piece::King, b.kings() & b.white(), occ),
        //     ]
        // } else {
        //     [
        //         bb.all_attacks(Color::Black, Piece::Pawn, b.pawns() & b.black(), occ),
        //         bb.all_attacks(Color::Black, Piece::Knight, b.knights() & b.black(), occ),
        //         bb.all_attacks(Color::Black, Piece::Bishop, b.bishops() & b.black(), occ),
        //         bb.all_attacks(Color::Black, Piece::Rook, b.rooks() & b.black(), occ),
        //         bb.all_attacks(Color::Black, Piece::Queen, b.queens() & b.black(), occ),
        //         bb.all_attacks(Color::Black, Piece::King, b.kings() & b.black(), occ),
        //     ]
        // };

        // let our_atts = atts[c];
        // let opp_atts = atts[c.opposite()];

        // let _opp_attacks_bb =
        //     opp_atts[0] | opp_atts[1] | opp_atts[2] | opp_atts[3] | opp_atts[4] | opp_atts[5];

        // let our_attacks_bb =
        //     our_atts[0] | our_atts[1] | our_atts[2] | our_atts[3] | our_atts[4] | our_atts[5];

        // // let controlled = (our_attacks_bb - opp_attacks_bb) & Bitboard::CENTER_16_SQ;
        // let defended =
        //     (our_attacks_bb & (b.line_pieces() | b.knights() | b.pawns()) & us).popcount();
        //  our_atts[1] | our_atts[2] | our_atts[3] | our_atts[4] | our_atts[5];
        // &us;

        // knights
        //
        let mut ni_atts = Bitboard::empty();
        for sq in (b.knights() & us).squares() {
            // let p = Piece::Knight;
            let our_raw_attacks = bb.knight_attacks(sq);
            // empty squares + undefended + defended qrkb (but not defended pawns)
            let our_attacks = our_raw_attacks - (pa & (empty | their_p));
            ni_atts |= our_raw_attacks;
            // | (them & !their_attacks_bb) | (bi | r | q | ni));

            center_attacks += (our_attacks & Bitboard::CENTER_16_SQ).popcount();
            knight_attacks_center += (our_attacks & Bitboard::CENTER_4_SQ).popcount();
            knight_connected |= (our_raw_attacks & b.knights() & us).any();
            if b.color_us() == c {
                for sq in our_attacks.squares() {
                    let atts = bb.knight_attacks(sq);
                    if (atts & them & (b.rooks_or_queens() | b.bishops() | b.kings())).two_or_more()
                    {
                        knight_forks += 1;
                    }
                }
            }
            if bb.pawn_attack_span(c, sq).disjoint(their_p)
                && sq.is_in(Bitboard::home_half(opponent))
                && sq.is_in(
                    Bitboard::FILE_C | Bitboard::FILE_D | Bitboard::FILE_E | Bitboard::FILE_F,
                )
            {
                if sq.is_in(our_pa) {
                    knight_outpost_pawn_defended += 1;
                }
            }
            // knight_trapped += (piece_move_squares + asym_attacks == 0) as i32;

            double_attacks |= our_attacks & them & all_attacks;
            all_attacks |= our_attacks & them;

            attacks_near_king +=
                (our_attacks & pawn_shield).any() as i32 * (Piece::Knight.centipawns() / 64);
            s.set_bits(AttacksNearKing, our_attacks & pawn_shield);
            let atts = our_attacks.popcount();
            if atts == 1 {
                _partially_trapped_pieces += 1;
            }
            if atts == 0 {
                _fully_trapped_pieces += 1;
            }
            // self.mv.push((p, our_attacks.popcount()));
            _move_squares += atts;
            let feat = match atts {
                0 | 1 => KnightMoves2,
                2 => KnightMoves3,
                3 | 4 => KnightMoves4,
                5 | 6 => KnightMoves5,
                7 | 8 => KnightMoves6,
                _ => unreachable!(),
            };
            s.accum2(c, feat, 1);
        }
        s.accum2(c, TempoKnightForks, knight_forks);
        s.accum2(c, KnightOutpost, knight_outpost);
        s.accum2(c, KnightOutpostPawnDefended, knight_outpost_pawn_defended);
        s.accum2(c, KnightOutpostRookSafe, knight_outpost_rook_safe);
        s.accum2(c, KnightConnected, knight_connected as i32);
        s.accum2(c, KnightAttacksCenter, knight_attacks_center);
        s.accum2(c, KnightTrapped, knight_trapped);

        // bishops
        //
        let mut bi_atts = Bitboard::empty();
        // let mut relative_pins_by_bishop = 0;
        // let mut discovered_atts_by_bishop = 0;
        for sq in (b.bishops() & us).squares() {
            let our_raw_attacks = bb.bishop_attacks(occ, sq);
            // let our_non_pin_attacks = bb.bishop_attacks(occ - ni, sq);
            // let our_discovered_attacks = bb.bishop_attacks(occ - us, sq);
            // if ((our_non_pin_attacks - our_raw_attacks) & (q|r)).any() {
            //     relative_pins_by_bishop += 1;
            // }
            // if ((our_discovered_attacks - our_raw_attacks) & (q|r)).any() {
            //     discovered_atts_by_bishop += 1;
            // }

            // empty squares + undefended + defended qrkb (but not defended pawns)

            let our_attacks = our_raw_attacks - (pa & (empty | their_p));
            bi_atts |= our_raw_attacks;
            // | (them & !their_attacks_bb) | (bi | r | q | ni));

            center_attacks += (our_attacks & Bitboard::CENTER_16_SQ).popcount();
            if bb.pawn_attack_span(c, sq).disjoint(their_p)
                && sq.is_in(Bitboard::home_half(opponent))
                && sq.is_in(our_pa)
            {
                bishop_outposts += 1;
            }
            double_attacks |= our_attacks & them & all_attacks;
            all_attacks |= our_attacks & them;

            attacks_near_king +=
                (our_attacks & pawn_shield).any() as i32 * (Piece::Bishop.centipawns() / 64);
            s.set_bits(AttacksNearKing, our_attacks & pawn_shield);
            let atts = our_attacks.popcount();
            if atts == 1 {
                _partially_trapped_pieces += 1;
            }
            if atts == 0 {
                _fully_trapped_pieces += 1;
            }
            // self.mv.push((p, our_attacks.popcount()));
            _move_squares += atts;
            let feat = match atts {
                0 | 1 => BishopMoves2,
                2 | 3 => BishopMoves3,
                4 | 5 | 6 => BishopMoves4,
                7 | 8 | 9 => BishopMoves5,
                10 | 11 | 12 | 13 | 14 => BishopMoves6,
                _ => unreachable!(),
            };
            s.accum2(c, feat, 1);
        }
        // s.accum2(c, RelativePinsByBishop, relative_pins_by_bishop);
        // s.accum2(c, DiscoveredAttsByBishop, discovered_atts_by_bishop);

        let bishop_color_pawns = |c: Color| {
            if (b.bishops() & b.color(c)).exactly_one() {
                if Bitboard::WHITE_SQUARES.contains(b.bishops() & b.color(c)) {
                    return (b.pawns() & b.color(c) & Bitboard::WHITE_SQUARES).popcount()
                        - (b.pawns() & b.color(c) & Bitboard::BLACK_SQUARES).popcount();
                } else if Bitboard::BLACK_SQUARES.contains(b.bishops() & b.color(c)) {
                    return (b.pawns() & b.color(c) & Bitboard::BLACK_SQUARES).popcount()
                        - (b.pawns() & b.color(c) & Bitboard::WHITE_SQUARES).popcount();
                }
            }
            0
        };
        s.accum(
            BishopColorPawns,
            bishop_color_pawns(White),
            bishop_color_pawns(Black),
        );
        s.accum2(c, BishopOutposts, bishop_outposts);
        s.accum2(c, BishopTrapped, bishop_trapped);

        // rooks
        //
        let mut ro_atts = Bitboard::empty();
        // let mut relative_pins_by_rook = 0;
        for sq in (b.rooks() & us).squares() {
            let our_raw_attacks = bb.rook_attacks(occ, sq);
            // let our_xray_attacks = bb.rook_attacks(occ - ((b.bishops() | b.knights()) & us), sq);
            // if ((our_xray_attacks - our_raw_attacks) & q).any() {
            //     relative_pins_by_rook += 1;
            // }

            let our_attacks = our_raw_attacks - (pa & (empty | their_p | bi | ni));
            ro_atts |= our_raw_attacks;
            //| (them & !their_attacks_bb) | (r | q));

            center_attacks += (our_attacks & Bitboard::CENTER_16_SQ).popcount();
            enemy_pawns_on_rook_rank +=
                (sq.rank() & b.pawns() & them & Bitboard::home_half(opponent)).popcount();
            double_attacks |= our_attacks & them & all_attacks;
            all_attacks |= our_attacks & them;

            attacks_near_king +=
                (our_attacks & pawn_shield).any() as i32 * (Piece::Rook.centipawns() / 64);
            s.set_bits(AttacksNearKing, our_attacks & pawn_shield);
            let atts = our_attacks.popcount();
            if atts == 1 {
                _partially_trapped_pieces += 1;
            }
            if atts == 0 {
                _fully_trapped_pieces += 1;
            }
            // self.mv.push((p, our_attacks.popcount()));
            _move_squares += atts;
            let feat = match atts {
                0 | 1 | 2 | 3 => RookMoves3,
                4 | 5 | 6 => RookMoves4,
                7 | 8 | 9 => RookMoves5,
                10 | 11 | 12 | 13 | 14 => RookMoves6,
                _ => unreachable!(),
            };
            s.accum2(c, feat, 1);
        }
        // s.accum2(c, RelativePinsByRook, relative_pins_by_rook);

        let doubled_rooks = ((b.rooks() & us).two_or_more()
            && (b.rooks() & us).first_square().file_index()
                == (b.rooks() & us).last_square().file_index()) as i32;
        let doubled_rooks_open_file =
            (doubled_rooks == 1 && (open_files & b.rooks() & us).popcount() >= 2) as i32;
        let rook_on_open_file = (open_files & us & b.rooks()).popcount();
        s.accum2(c, RookOpenFile, rook_on_open_file);

        let rook_semi_open_file = (semi_open_files & us & b.rooks()).popcount();
        s.accum2(c, RookSemiOpenFile, rook_semi_open_file);

        s.set_bits(RookOpenFile, open_files & us & b.rooks());
        s.accum2(c, ConnectedRooks, connected_rooks as i32);
        s.accum2(c, DoubledRooks, doubled_rooks);
        s.accum2(c, DoubledRooksOpenFile, doubled_rooks_open_file);
        s.accum2(c, EnemyPawnsOnRookRank, enemy_pawns_on_rook_rank);
        s.accum2(c, RookTrapped, rook_trapped);

        // queens
        //
        let mut qu_atts = Bitboard::empty();
        for sq in (b.queens() & us).squares() {
            let our_raw_attacks = bb.rook_attacks(occ, sq) | bb.bishop_attacks(occ, sq);

            let our_attacks = our_raw_attacks - (pa & (empty | their_p | bi | ni | r));
            qu_atts |= our_raw_attacks;
            //  | (them & !their_attacks_bb) | q);

            center_attacks += (our_attacks & Bitboard::CENTER_16_SQ).popcount();
            double_attacks |= our_attacks & them & all_attacks;
            all_attacks |= our_attacks & them;

            attacks_near_king +=
                (our_attacks & pawn_shield).any() as i32 * (Piece::Queen.centipawns() / 64);
            s.set_bits(AttacksNearKing, our_attacks & pawn_shield);

            let atts = our_attacks.popcount();
            if atts == 1 {
                _partially_trapped_pieces += 1;
            }
            if atts == 0 {
                _fully_trapped_pieces += 1;
            }
            // self.mv.push((p, our_attacks.popcount()));
            _move_squares += atts;
            let feat = match atts {
                0..=6 => QueenMoves4,
                7 | 8 | 9 | 10 | 11 => QueenMoves5,
                _ => QueenMoves6,
            };
            s.accum2(c, feat, 1);
        }
        s.accum2(c, QueenOpenFile, queens_on_open_files);
        s.accum2(c, QueenTrapped, queen_trapped);

        // let ki_atts = bb.king_attacks((b.kings() & us).square());
        // let our_attacks_bb = ni_atts | bi_atts | ro_atts | qu_atts | ope | opw | ki_atts;

        // let controlled = (our_attacks_bb - opp_attacks_bb) & Bitboard::CENTER_16_SQ;
        // let defended = our_attacks_bb & (b.line_pieces() | b.knights() | b.pawns() ) & us;

        //  let attacked = our_attacks_bb & self.pawn_structure.weak & them;

        // s.accum(c, Controlled, controlled.popcount());
        // s.accum(c, Defended, defended.popcount());
        // s.accum(c, Attacked, attacked.popcount());

        // all
        //
        s.accum2(c, AttacksNearKing, attacks_near_king);
        s.accum2(c, CenterAttacks, center_attacks);

        s.set_bits(DoubleAttacks, double_attacks);
        s.accum2(c, DoubleAttacks, double_attacks.popcount());
        // s.accum(
        //     c,
        //     DoubleAttacksNearKing,
        //     double_attacks_near_king.popcount(),
        // );
        // s.accum(c, MovesNearKing, moves_near_king);

        // s.accum(c, UndefendedSq, move_squares);
        // s.accum(c, UndefendedPiece, 0);
        // s.accum(c, TrappedPiece, fully_trapped_pieces);
        // s.accum(
        //     c,
        //     PartiallyTrappedPiece,
        //     partially_trapped_pieces,
        // );

        // for sq in ((b.rooks() | b.bishops() | b.queens()) & us).squares() {
        //     let p = b.piece_unchecked(sq);

        //     // non-pawn-defended empty or oppoent sq
        //     // include "attacking" our own pieces
        //     let our_raw_attacks = bb.attacks(c, p, Bitboard::empty(), occ, sq);

        //     let our_attacks = our_raw_attacks - us - pa;
        //     center_attacks += (our_attacks & Bitboard::CENTER_16_SQ).popcount();

        //     let piece_move_squares = (our_attacks - occ).popcount();

        //     // those attacks on enemy that arent pawn defended and cant attack back
        //     let asym_attacks;
        //     match p {
        //         Piece::Rook => {
        //             // connected_rooks |= (our_raw_attacks & b.rooks() & us).any();
        //             enemy_pawns_on_rook_rank +=
        //                 (sq.rank() & b.pawns() & them & Bitboard::home_half(opponent)).popcount()
        //                     as i32;
        //             asym_attacks = ((our_attacks & them) - r).popcount();
        //             // rook_trapped += (piece_move_squares + asym_attacks == 0) as i32;
        //         }
        //         Piece::Queen => {
        //             asym_attacks = (our_attacks & them).popcount();
        //             // queen_trapped += (piece_move_squares + asym_attacks == 0) as i32;
        //         }
        //         _ => unreachable!(),
        //     };
        //     // trapped piece
        //     if piece_move_squares + asym_attacks == 1 {
        //         partially_trapped_pieces += 1;
        //     }
        //     if piece_move_squares == 0 {
        //         fully_trapped_pieces += 1;
        //     }
        //     // self.mv.push((p, our_attacks.popcount()));
        //     move_squares += piece_move_squares;
        //     non_pawn_defended_moves += asym_attacks;

        //     double_attacks |= ((our_raw_attacks & them) - us) & all_attacks;
        //     all_attacks |= (our_raw_attacks & them) - us;

        //     // moves_near_king += (our_raw_attacks
        //     //     & bb.within_chebyshev_distance_inclusive(ksq, 1)
        //     //     & !b.occupied())
        //     // .popcount();
        //     attacks_near_king +=
        //         (our_raw_attacks & pawn_shield & !us).any() as i32 * (p.centipawns() / 64);
        //     // * match p {
        //     //     Piece::Bishop => 10,
        //     //     Piece::Knight => 20,
        //     //     Piece::Rook => 30,
        //     //     Piece::Queen => 50,
        //     //     _ => unreachable!(),
        //     // };
        //     s.set_bits(AttacksNearKing, our_raw_attacks & pawn_shield);
        // }

        // let double_attacks_near_king = double_attacks & bb.within_chebyshev_distance_inclusive(ksq, 1);
        //
        // knight
        //

        // Bishop
        //
        // TODO:BishopPawnTrap
        // https://www.chessprogramming.org/Trapped_Pieces
        // bishop prison (white bishop on A7 trapped by pawns on B6 & C7)
        //
        // const B_PAWNS_L: Bitboard = Bitboard::B6.or(Bitboard::C7);
        // const W_BISHOP_L: Bitboard = Bitboard::A7;
        // const B_PAWNS_R: Bitboard = B_PAWNS_L.flip_horizontal();
        // const W_BISHOP_R: Bitboard = W_BISHOP_L.flip_horizontal();
        // let l = (b.bishops() & us & c.chooser_wb(W_BISHOP_L, W_BISHOP_L.flip_vertical())).any()
        //     && (b.pawns() & them).contains(c.chooser_wb(B_PAWNS_L, B_PAWNS_L.flip_vertical()));
        // let r = (b.bishops() & us & c.chooser_wb(W_BISHOP_R, W_BISHOP_R.flip_vertical())).any()
        //     && (b.pawns() & them).contains(c.chooser_wb(B_PAWNS_R, B_PAWNS_R.flip_vertical()));
        // let bishop_pawn_trap = (l as i32) + (r as i32);
        // s.accum(c, BishopPawnTrap, bishop_pawn_trap);

        // Rook
        //
    }
}

#[cfg(test)]
mod eval_tests {
    use super::*;
    use crate::{
        catalog::Catalog,
        eval::{
            eval::WeightsKind, score::WhiteScore, scoring::Hardcoded, weight::WeightOf,
            CategoryScorer, ExplainingScorer, Softcoded, SummationScorer, WeightVec,
        },
        infra::{black_box, profiler::PerfProfiler},
        other::{Perft, Phase},
        test_log::test,
        Algo,
    };

    #[test]
    fn test_eval_basics() {
        let phase = Phase(50);
        let b = &Catalog::test_position().board().clone();
        let mut explain = ExplainingScorer::new(b.clone(), phase, |f| Hardcoded::<f32>::WTS[f]);
        Eval.eval(b, &mut explain);
        println!("{explain}");

        let mut sum = SummationScorer::new(|f| Hardcoded::<f32>::WTS[f]);
        let b = &Catalog::test_position().board().clone();
        Eval.eval(b, &mut sum);
        // sum.eval(b, &mut |f, w, b| {
        //     total += (w - b) * HardcodedF32Wts::weight(f)
        // });
        println!("{}", sum.total);
    }

    #[test]
    fn bench_evals() {
        // let mut totalf32 = WeightOf::<f32>::zero();
        // let mut totali32 = WeightOf::<i32>::zero();
        let mut starting_pos = Catalog::perft_kiwipete().0;
        // let mut sum_i32 = SummationScorer::<i32>::default();
        // let mut sum_f32 = SummationScorer::<f32>::default();

        let mut algo = Algo::from_config();
        algo.eval.mb.enabled = false;
        let old_eval = &mut algo.eval;

        let soft_i32_wts = Softcoded {
            wts: Hardcoded::<i32>::WTS,
        };
        let soft_i16_wts = Softcoded {
            wts: Hardcoded::<i32>::WTS.map(|wt| WeightOf::new(wt.0 as i16, wt.1 as i16)),
        };
        let soft_f32_wts = Softcoded {
            wts: Hardcoded::<f32>::WTS,
        };
        let soft_f64_wts = Softcoded {
            wts: Hardcoded::<f32>::WTS.map(|n| WeightOf::<f64>(n.0 as f64, n.1 as f64)),
        };
        let mut prof_cat_f32 = PerfProfiler::new("eval: cat soft f32".into());
        let mut prof_hard_i32 = PerfProfiler::new("eval: hard i32".into());
        // let mut prof_hard_i32 = PerfProfiler::new("eval: hard i32".into());
        let mut prof_hard_f32 = PerfProfiler::new("eval: hard f32".into());
        let mut prof_soft_i32 = PerfProfiler::new("eval: soft i32".into());
        let mut prof_soft_i16 = PerfProfiler::new("eval: soft i16".into());
        let mut prof_soft_f32 = PerfProfiler::new("eval: soft f32".into());
        let mut prof_soft_f64 = PerfProfiler::new("eval: soft f64".into());
        let mut prof_old_f32 = PerfProfiler::new("eval: old".into());
        let mut prof_new_f32 = PerfProfiler::new("eval: new f32".into());
        let mut prof_new_f64 = PerfProfiler::new("eval: new f64".into());
        let mut prof_new_i32 = PerfProfiler::new("eval: new i32".into());
        let mut prof_new_hci32 = PerfProfiler::new("eval: new hc i32".into());
        let mut prof_new_hci32_mp = PerfProfiler::new("eval: new hc i32 mp".into());

        // old_eval.init();

        let mut func = |b: &Board, mv: Move| {
            let b = &mut b.make_move(mv);
            b.calculate_internals();
            prof_hard_f32.benchmark(|| {
                let mut sum = SummationScorer::new(|f| Hardcoded::<f32>::WTS[f]);
                Eval.eval(black_box(b), &mut sum)
            });
            b.calculate_internals();
            prof_hard_i32.benchmark(|| {
                let mut sum = SummationScorer::new(|f| Hardcoded::<i32>::WTS[f]);
                Eval.eval(black_box(b), &mut sum)
            });
            b.calculate_internals();
            prof_soft_i32.benchmark(|| {
                let mut sum = SummationScorer::new(|f: Feature| soft_i32_wts.wts[f]);
                Eval.eval(black_box(b), &mut sum)
            });
            b.calculate_internals();
            prof_soft_i16.benchmark(|| {
                let mut sum = SummationScorer::new(|f: Feature| soft_i16_wts.wts[f]);
                Eval.eval(black_box(b), &mut sum)
            });
            b.calculate_internals();
            prof_soft_f64.benchmark(|| {
                let mut sum = SummationScorer::new(|f: Feature| soft_f64_wts.wts[f]);
                Eval.eval(black_box(b), &mut sum)
            });
            b.calculate_internals();
            prof_soft_f32.benchmark(|| {
                let mut sum = SummationScorer::new(|f: Feature| soft_f32_wts.wts[f]);
                Eval.eval(black_box(b), &mut sum)
            });
            b.calculate_internals();
            prof_cat_f32.benchmark(|| {
                let mut sum = CategoryScorer::<f32, _>::new(|f| soft_f32_wts.wts[f]);
                Eval.eval(black_box(b), &mut sum)
            });
            b.calculate_internals();
            old_eval.weights_kind = WeightsKind::Legacy;
            prof_old_f32.benchmark(|| old_eval.w_eval_no_cache(black_box(b)));

            b.calculate_internals();
            old_eval.weights_kind = WeightsKind::SoftcodedF64;
            prof_new_f64.benchmark(|| old_eval.w_eval_no_cache(black_box(b)));

            b.calculate_internals();
            old_eval.weights_kind = WeightsKind::SoftcodedI32;
            prof_new_i32.benchmark(|| old_eval.w_eval_no_cache(black_box(b)));

            b.calculate_internals();
            old_eval.weights_kind = WeightsKind::HardcodedI32;
            prof_new_hci32.benchmark(|| old_eval.w_eval_no_cache(black_box(b)));

            b.calculate_internals();
            old_eval.weights_kind = WeightsKind::HardcodedI32Millis;
            prof_new_hci32_mp.benchmark(|| old_eval.w_eval_no_cache(black_box(b)));

            b.calculate_internals();
            old_eval.weights_kind = WeightsKind::SoftcodedF32;
            prof_new_f32.benchmark(|| old_eval.w_eval_no_cache(black_box(b)));
        };
        Perft::perft_with(&mut starting_pos, 2, &mut func);
    }

    #[test]

    fn compare_evals() -> anyhow::Result<()> {
        use std::io::Write;
        let mut starting_pos = Catalog::perft_kiwipete().0;
        let algo = Algo::from_config();
        let old_eval = &algo.eval;
        let soft_f32_wts = Softcoded {
            wts: Hardcoded::<f32>::WTS,
        };

        let mut tw = tabwriter::TabWriter::new(std::io::stdout());
        let mut func = |b: &Board, _mv: Move| {
            let mut sum = SummationScorer::new(|f| soft_f32_wts.wts[f]);
            Eval.eval(b, &mut sum);
            let phase = b.phase(&algo.eval.phaser);
            let new_score = WhiteScore(Score::from_f32(sum.total.interpolate(phase)));
            let old_score = old_eval.w_eval_no_cache(black_box(b));

            if (new_score.as_white_cp() - old_score.as_white_cp()).abs() > 1 {
                writeln!(tw, "Old score: {old_score}").unwrap();
                writeln!(tw, "New score: {new_score}").unwrap();
                let mut explain_n =
                    ExplainingScorer::new(b.clone(), phase, |f| soft_f32_wts.wts[f]);
                let explain_o = old_eval.w_eval_explain(b);
                Eval.eval(b, &mut explain_n);
                writeln!(tw, "\nExplain new follows...\n{}\n\n", &explain_n).unwrap();
                writeln!(tw, "\nExplain old follows...\n{explain_o}\n\n").unwrap();
                writeln!(
                    tw,
                    "\nWeights vector...\n{:?}\n\n",
                    &soft_f32_wts as &dyn WeightVec<f32>
                )
                .unwrap();
                assert_eq!(new_score, old_score);
            }
        };
        Perft::perft_with(&mut starting_pos, 3, &mut func);
        tw.flush()?;
        Ok(())
    }
}
