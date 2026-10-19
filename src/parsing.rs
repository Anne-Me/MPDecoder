use std::fs::File;
use std::io::{BufReader, Error, BufRead};
use std::vec;
use sscanf::sscanf;

use crate::tanner_graph::TannerGraph;



pub fn alist_to_tanner(file_path: &str) -> Result<TannerGraph, Error> {
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    let mut line_iter = reader.lines();

    let mut next_line = line_iter.next().expect("Missing first line.").unwrap();
    let (n,m) = sscanf!(next_line, "{} {}", usize, usize).expect("Error parsing first line.");

    next_line = line_iter.next().expect("Missing second line").unwrap();
    let (dvmax,dcmax) = sscanf!(next_line, "{} {}", usize, usize).expect("Error parsing max variable node degree.");

    // read n integers from line
    next_line = line_iter.next().expect("Missing third line").unwrap();
    let col_weights: Vec<usize> = collect_ints(&next_line);

    // read m integers from line
    next_line = line_iter.next().expect("Missing fourth line").unwrap();
    let row_weights: Vec<usize> = collect_ints(&next_line);

    // create tanner graph with n,m
    let mut tanner = TannerGraph {
        n: n,
        m: m,
        v_nodes: vec![Vec::new();n],
        c_nodes: vec![Vec::new();m],
    };


    for i in 0..n {
        next_line = line_iter.next().expect("Missing line").unwrap();
        // the numbers indicate which position in the ith row is nonzero, i.e. incoming edges to check node i
        let v: Vec<usize> = collect_ints(&next_line);

        for elem in v {
            tanner.v_nodes[i].push(elem); // node are stored starting at 0, elems are starting at 1
        }
    }

    for i in 0..m {
        next_line = line_iter.next().expect("Missing line").unwrap();
        // the numbers indicate which position in the ith col is nonzero, i.e. edges to variable node i
        let v: Vec<usize> = collect_ints(&next_line);

        for elem in v {
            tanner.c_nodes[i].push(elem); // node are stored starting at 0, elems are starting at 1
        }
    }


    return Ok(tanner)
}

fn collect_ints(next_line: &str) -> Vec<usize> {
    return next_line.split_whitespace()
        .map(|value| {
            value
            .parse::<usize>()
            .expect("Not an integer")
        })
        .take_while(|&value| {value != 0})
        .map(|value| {value - 1})// start indexing at 0 to make vector indexing easier in decoding
        .collect::<Vec<usize>>()
}

#[cfg(test)]
mod test {
    use std::assert_eq;
    use super::*;

    #[test]
    fn test_parsing_hamming() {
        let tanner = alist_to_tanner(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/testfiles/hamming74.txt")).expect("error");
        assert_eq!(tanner.v_nodes.len(), 7);
        assert_eq!(tanner.c_nodes.len(), 3);
        assert_eq!(tanner.c_nodes[0], [0,1,3,4]);
        assert_eq!(tanner.c_nodes[1], [0,2,3,5]);
        assert_eq!(tanner.c_nodes[2], [1,2,3,6]);
        assert_eq!(tanner.v_nodes[0], [0,1]);
        assert_eq!(tanner.v_nodes[1], [0,2]);
        assert_eq!(tanner.v_nodes[2], [1,2]);
        assert_eq!(tanner.v_nodes[3], [0,1,2]);
        assert_eq!(tanner.v_nodes[4], [0]);
        assert_eq!(tanner.v_nodes[5], [1]);
        assert_eq!(tanner.v_nodes[6], [2]);
    }

    #[test]
    fn test_parsing_gallager_1(){
        let tanner = alist_to_tanner(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/testfiles/96-3-963")).expect("error");
        assert_eq!(tanner.v_nodes.len(), 96);
        assert_eq!(tanner.c_nodes.len(), 48);
        assert_eq!(tanner.v_nodes[0], [9,29,39]);
        assert_eq!(tanner.v_nodes[49], [3,22,43]);
        assert_eq!(tanner.v_nodes[95], [9,18,33]);
        assert_eq!(tanner.c_nodes[0], [7,19,35,55,79,80]);
        assert_eq!(tanner.c_nodes[23], [6,28,32,61,68,94]);
        assert_eq!(tanner.c_nodes[47], [12,23,41,61,79,93]);
    }

    #[test]
    fn test_parsing_gallager_2(){
        let tanner = alist_to_tanner(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/testfiles/816-4-383")).expect("error");
        assert_eq!(tanner.v_nodes.len(), 816);
        assert_eq!(tanner.c_nodes.len(), 408);
        assert_eq!(tanner.v_nodes[0], [82,144,298,394]);
        assert_eq!(tanner.v_nodes[409], [18,193,269,375]);
        assert_eq!(tanner.v_nodes[815], [34,183,303,338]);
        assert_eq!(tanner.c_nodes[0], [74,172,208,353,411,568,647,753]);
        assert_eq!(tanner.c_nodes[203], [49,151,304,378,477,610,623,754]);
        assert_eq!(tanner.c_nodes[407], [55,159,264,381,430,550,628,806]);
    }

    #[test]
    fn test_parsing_hamming_with_zeros(){
        let tanner = alist_to_tanner(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/testfiles/hamming74_padded.txt")).expect("error");
        assert_eq!(tanner.v_nodes.len(), 7);
        assert_eq!(tanner.c_nodes.len(), 3);
        assert_eq!(tanner.c_nodes[0], [0,1,3,4]);
        assert_eq!(tanner.c_nodes[1], [0,2,3,5]);
        assert_eq!(tanner.c_nodes[2], [1,2,3,6]);
        assert_eq!(tanner.v_nodes[0], [0,1]);
        assert_eq!(tanner.v_nodes[1], [0,2]);
        assert_eq!(tanner.v_nodes[2], [1,2]);
        assert_eq!(tanner.v_nodes[3], [0,1,2]);
        assert_eq!(tanner.v_nodes[4], [0]);
        assert_eq!(tanner.v_nodes[5], [1]);
        assert_eq!(tanner.v_nodes[6], [2]);
    }
}