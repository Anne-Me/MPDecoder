use std::fs::File;
use std::io::{BufReader, Error, BufRead};
use std::vec;
use sscanf::sscanf;

use crate::tanner_graph::TannerGraph;



pub fn alist_to_tanner(file_path: &str) -> Result<TannerGraph, Error> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut line_iter = reader.lines();

    let mut next_line = line_iter.next().expect("Missing first line.").unwrap();
    let (n,m) = sscanf!(next_line, "{} {}", usize, usize).expect("Error parsing first line.");

    next_line = line_iter.next().expect("Missing second line").unwrap();
    let (_dvmax,_dcmax) = sscanf!(next_line, "{} {}", usize, usize).expect("Error parsing max variable node degree.");

    // read n integers from line
    next_line = line_iter.next().expect("Missing third line").unwrap();
    let _col_weights: Vec<usize> = collect_ints(&next_line);

    // read m integers from line
    next_line = line_iter.next().expect("Missing fourth line").unwrap();
    let _row_weights: Vec<usize> = collect_ints(&next_line);

    // create tanner graph with n,m
    let mut tanner = TannerGraph {
        n,
        m,
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


    Ok(tanner)
}

fn collect_ints(next_line: &str) -> Vec<usize> {
    next_line.split_whitespace()
        .map(|value| {
            value
            .parse::<usize>()
            .expect("Not an integer")
        })
        .take_while(|&value| {value != 0})
        .map(|value| {value - 1})// start indexing at 0 to make vector indexing easier in decoding
        .collect::<Vec<usize>>()
}

// detector error model (dem) as written by stim, each error mechanism is one variable node
pub struct Dem {
    pub tanner: TannerGraph,          // check nodes = detectors, variable nodes = error mechanisms
    pub priors: Vec<f64>,             // probability of each error mechanism
    pub observables: Vec<Vec<usize>>, // logical observables flipped by each error mechanism
}

// one instruction of a dem file, detector ids are still relative to shift_detectors
enum DemInst {
    Error { p: f64, dets: Vec<usize>, obs: Vec<usize> },
    Shift(usize),
    Repeat(usize, Vec<DemInst>),
}

pub fn parse_dem(file_path: &str) -> Result<Dem, String> {
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("could not read dem file '{file_path}': {e}"))?;
    // everything after '#' is a comment
    let mut lines = content
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.split('#').next().unwrap().trim()));
    let insts = parse_dem_block(&mut lines, false).map_err(|e| format!("dem file '{file_path}', {e}"))?;

    let mut builder = DemBuilder::default();
    builder.run(&insts);

    // check node c is connected to every error mechanism that flips detector c
    let mut c_nodes = vec![Vec::new(); builder.num_detectors];
    for (v, dets) in builder.columns.iter().enumerate() {
        for &d in dets {
            c_nodes[d].push(v);
        }
    }
    let tanner = TannerGraph { n: builder.columns.len(), m: builder.num_detectors, v_nodes: builder.columns, c_nodes };
    Ok(Dem { tanner, priors: builder.priors, observables: builder.observables })
}

// parses instructions until the end of the file or, inside a repeat block, until the closing '}'
fn parse_dem_block<'a>(lines: &mut impl Iterator<Item = (usize, &'a str)>, in_repeat: bool) -> Result<Vec<DemInst>, String> {
    let mut insts = Vec::new();
    while let Some((line_nr, line)) = lines.next() {
        if line.is_empty() {
            continue;
        }
        if line == "}" {
            return if in_repeat { Ok(insts) } else { Err(format!("line {line_nr}: '}}' without a repeat block")) };
        }
        let (name, args, targets) = split_instruction(line).map_err(|e| format!("line {line_nr}: {e}"))?;
        if name == "repeat" {
            let count = targets.strip_suffix('{').and_then(|c| c.trim().parse().ok())
                .ok_or(format!("line {line_nr}: expected 'repeat <count> {{', got '{line}'"))?;
            insts.push(DemInst::Repeat(count, parse_dem_block(lines, true)?));
            continue;
        }
        if let Some(inst) = parse_dem_instruction(name, args, targets).map_err(|e| format!("line {line_nr}: {e}"))? {
            insts.push(inst);
        }
    }
    if in_repeat { Err("repeat block is missing its closing '}'".to_string()) } else { Ok(insts) }
}

// None for instructions that do not change the graph
fn parse_dem_instruction(name: &str, args: &str, targets: &str) -> Result<Option<DemInst>, String> {
    match name {
        "error" => parse_dem_error(args, targets).map(Some),
        "shift_detectors" if targets.is_empty() => Ok(None),
        "shift_detectors" => targets.parse().map(|k| Some(DemInst::Shift(k))).map_err(|_| format!("invalid shift '{targets}'")),
        // only declare detectors/observables with coordinates, not needed for decoding
        "detector" | "logical_observable" => Ok(None),
        _ => Err(format!("unknown instruction '{name}'")),
    }
}

// splits 'name[tag](args) targets', the tag is ignored
fn split_instruction(line: &str) -> Result<(&str, &str, &str), String> {
    let name_end = line.find(['[', '(', ' ', '\t']).unwrap_or(line.len());
    let (name, mut rest) = line.split_at(name_end);
    if let Some(r) = rest.strip_prefix('[') {
        rest = &r[r.find(']').ok_or("tag is missing its closing ']'")? + 1..];
    }
    let mut args = "";
    if let Some(r) = rest.strip_prefix('(') {
        let end = r.find(')').ok_or("arguments are missing their closing ')'")?;
        args = &r[..end];
        rest = &r[end + 1..];
    }
    Ok((name, args, rest.trim()))
}

fn parse_dem_error(args: &str, targets: &str) -> Result<DemInst, String> {
    let p: f64 = args.trim().parse().map_err(|_| format!("invalid probability '{args}'"))?;
    if !(0.0..=1.0).contains(&p) {
        return Err(format!("probability must be between 0 and 1, got {p}"));
    }
    let (mut dets, mut obs) = (Vec::new(), Vec::new());
    for t in targets.split_whitespace() {
        // '^' separates the parts of a decomposed error, BP decodes the error as a whole
        if t == "^" {
            continue;
        }
        let list = if t.starts_with('D') { &mut dets } else if t.starts_with('L') { &mut obs } else {
            return Err(format!("invalid target '{t}'"));
        };
        let id = parse_dem_id(t)?;
        // a target that appears twice cancels out
        match list.iter().position(|&x| x == id) {
            Some(pos) => { list.remove(pos); }
            None => list.push(id),
        }
    }
    Ok(DemInst::Error { p, dets, obs })
}

// 'D12' -> 12, 'L0' -> 0
fn parse_dem_id(t: &str) -> Result<usize, String> {
    t[1..].parse().map_err(|_| format!("invalid target '{t}'"))
}

// unrolls the instructions into columns, detector ids become absolute
#[derive(Default)]
struct DemBuilder {
    offset: usize,                // sum of all shift_detectors so far
    num_detectors: usize,         // largest absolute detector id + 1
    columns: Vec<Vec<usize>>,     // detectors flipped by each error mechanism
    priors: Vec<f64>,
    observables: Vec<Vec<usize>>,
}

impl DemBuilder {
    fn run(&mut self, insts: &[DemInst]) {
        for inst in insts {
            match inst {
                DemInst::Error { p, dets, obs } => {
                    let dets: Vec<usize> = dets.iter().map(|d| d + self.offset).collect();
                    self.num_detectors = dets.iter().fold(self.num_detectors, |m, &d| m.max(d + 1));
                    self.columns.push(dets);
                    self.priors.push(*p);
                    self.observables.push(obs.clone());
                }
                DemInst::Shift(k) => self.offset += k,
                DemInst::Repeat(count, body) => (0..*count).for_each(|_| self.run(body)),
            }
        }
    }
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