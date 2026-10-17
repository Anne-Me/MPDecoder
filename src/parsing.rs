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
        assert_eq!(tanner.c_nodes[0], [1,2,4,5]);
        assert_eq!(tanner.c_nodes[1], [1,3,4,6]);
        assert_eq!(tanner.c_nodes[2], [2,3,4,7]);
        assert_eq!(tanner.v_nodes[0], [1,2]);
        assert_eq!(tanner.v_nodes[1], [1,3]);
        assert_eq!(tanner.v_nodes[2], [2,3]);
        assert_eq!(tanner.v_nodes[3], [1,2,3]);
        assert_eq!(tanner.v_nodes[4], [1]);
        assert_eq!(tanner.v_nodes[5], [2]);
        assert_eq!(tanner.v_nodes[6], [3]);
    }

    #[test]
    fn test_parsing_gallager_1(){
        let tanner = alist_to_tanner(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/testfiles/96-3-963")).expect("error");
        assert_eq!(tanner.v_nodes.len(), 96);
        assert_eq!(tanner.c_nodes.len(), 48);
        assert_eq!(tanner.v_nodes[0], [10,30,40]);
        assert_eq!(tanner.v_nodes[49], [4,23,44]);
        assert_eq!(tanner.v_nodes[95], [10,19,34]);
        assert_eq!(tanner.c_nodes[0], [8,20,36,56,80,81]);
        assert_eq!(tanner.c_nodes[23], [7,29,33,62,69,95]);
        assert_eq!(tanner.c_nodes[47], [13,24,42,62,80,94]);
    }

    #[test]
    fn test_parsing_gallager_2(){
        let tanner = alist_to_tanner(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/testfiles/816-4-383")).expect("error");
        assert_eq!(tanner.v_nodes.len(), 816);
        assert_eq!(tanner.c_nodes.len(), 408);
        assert_eq!(tanner.v_nodes[0], [83,145,299,395]);
        assert_eq!(tanner.v_nodes[409], [19,194,270,376]);
        assert_eq!(tanner.v_nodes[815], [35,184,304,339]);
        assert_eq!(tanner.c_nodes[0], [75,173,209,354,412,569,648,754]);
        assert_eq!(tanner.c_nodes[203], [50,152,305,379,478,611,624,755]);
        assert_eq!(tanner.c_nodes[407], [56,160,265,382,431,551,629,807]);
    }

    #[test]
    fn test_parsing_hamming_with_zeros(){
        let tanner = alist_to_tanner(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/testfiles/hamming74_padded.txt")).expect("error");
        assert_eq!(tanner.v_nodes.len(), 7);
        assert_eq!(tanner.c_nodes.len(), 3);
        assert_eq!(tanner.c_nodes[0], [1,2,4,5]);
        assert_eq!(tanner.c_nodes[1], [1,3,4,6]);
        assert_eq!(tanner.c_nodes[2], [2,3,4,7]);
        assert_eq!(tanner.v_nodes[0], [1,2]);
        assert_eq!(tanner.v_nodes[1], [1,3]);
        assert_eq!(tanner.v_nodes[2], [2,3]);
        assert_eq!(tanner.v_nodes[3], [1,2,3]);
        assert_eq!(tanner.v_nodes[4], [1]);
        assert_eq!(tanner.v_nodes[5], [2]);
        assert_eq!(tanner.v_nodes[6], [3]);
    }
}