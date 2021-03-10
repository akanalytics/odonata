use crate::board::{Board, Move};
use crate::types::{Color, Piece};
use crate::board::makemove::MoveMaker;
use crate::eval::Scorable;
use crate::board::movegen::MoveGen;
use std::cmp;

// CPW
//
// Obligatory
//   Transposition Table
//   Iterative Deepening
//   Aspiration Windows
//
// Selectivity
//   Quiescence Search
//     static exchange evaluation < 0
//     delta pruning
//     standing pat

//   Selectivity
//   Mate Search
//
// Scout and Friends
//   Scout
//   NegaScout
//   Principal Variation Search
//
// Alpha-Beta goes Best-First
//   NegaC*
//   MTD(f)
//   Alpha-Beta Conspiracy Search
//


// taken from wikipedia
//
// function alphabeta(node, depth, α, β, maximizingPlayer) is
//     if depth = 0 or node is a terminal node then
//         return the heuristic value of node
//     if maximizingPlayer then
//         value := −∞
//         for each child of node do
//             value := max(value, alphabeta(child, depth − 1, α, β, FALSE))
//             α := max(α, value)
//             if α ≥ β then
//                 break (* β cutoff *)
//         return value
//     else
//         value := +∞
//         for each child of node do
//             value := min(value, alphabeta(child, depth − 1, α, β, TRUE))
//             β := min(β, value)
//             if β ≤ α then
//                 break (* α cutoff *)
//         return value
//
#[derive(Debug)]
pub struct Node<'b> {
    board: &'b Board,
    ply: u32,
    alpha: i32,
    beta: i32,
    score: i32,
    best_move: Move,
    // stats
    // leaf
    // pv
}


impl<'a> Node<'a> {

    pub fn is_maximizing(&self) -> bool {
        self.ply % 2 == 1  // 1 ply is just our moves scored
    }

    pub fn is_leaf(&self) -> bool {
        self.ply == 100
    }

    pub fn new_child<'b>(&self, board: &'b Board) -> Node<'b> {
        let mut child = Node {
            board, 
            alpha: self.alpha, 
            beta: self.beta, 
            ply: self.ply + 1, 
            score: 0,
            best_move: Default::default()
        };
        child.score = if child.is_maximizing() { i32::MIN } else { i32::MAX };
        debug_assert!(child.beta > child.alpha);
        child
    }


    pub fn alphabeta(&mut self) {
        if self.is_leaf() { 
            self.score = self.board.evaluate().total();
            return;
        }
        for mv in self.board.legal_moves().iter() {
            let board2 = self.board.make_move(mv);
            let mut child = self.new_child(&board2);
            child.alphabeta();
            let is_cut = self.process_child(mv, &child);
            if is_cut {
                break
            }
        }
        // end node
    }
    
    
    pub fn process_child(&mut self, mv: &Move,  child: &Node) -> bool {
        if self.is_maximizing() {
            if child.score > self.score {
                self.score = child.score;
                self.best_move = *mv;  // FIXME: copy size?
            } 
            self.alpha = cmp::max(self.alpha, child.score);
        } else {
            if child.score < self.score {
                self.score = child.score;
                self.best_move = *mv;
            } 
            self.beta = cmp::min(self.beta, child.score);
        }
        self.alpha >= self.beta
    }
}



pub struct Search {
    // Eval
    // Search config
    // Time controls
    // Transposition table
}


// impl Search {

//     pub fn new() -> Search {
//         Search
//     }




//     pub fn abort(&mut self) {

//     }

// }

