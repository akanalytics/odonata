use crate::board::makemove::MoveMaker;
use crate::board::Board;
use crate::catalog::Catalog;
use crate::movelist::MoveValidator;
use crate::movelist::{Move, MoveList};
use crate::outcome::GameEnd;
use crate::outcome::Outcome;
use crate::search::algo::Algo;
use std::fmt;
use std::time;


// https://www.chessprogramming.org/Extended_Position_Description

pub struct Epd { 
    board: Board,
    
}