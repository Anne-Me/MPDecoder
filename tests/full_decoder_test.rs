use std::vec;

use mp_decoder::binary_decoding::{Decoder, DecoderRule};
use mp_decoder::binary_decoding_quantum::QuantumDecoder;
use mp_decoder::parsing::alist_to_tanner;
use mp_decoder::tanner_graph::Bit;

#[test]
fn log_ratio_decode_96_3() {
    let tanner = alist_to_tanner(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/testfiles/96-3-963"
    ))
    .expect("96-3-963 should parse");
    assert_eq!((tanner.n, tanner.m), (96, 48));

    // syndrome
    let syndrome: Vec<Bit> = "000010000000000001011000000000000001010010000000"
        .bytes()
        .map(|bit| match bit {
            b'0' => Bit::ZERO,
            b'1' => Bit::ONE,
            _ => panic!("syndrome must contain only binary digits"),
        })
        .collect();

    // expected correction
    let mut exp_correction = vec![Bit::ZERO; tanner.n];
    exp_correction[26] = Bit::ONE;
    exp_correction[82] = Bit::ONE;
    exp_correction[87] = Bit::ONE;


    let decoder = Decoder::new(tanner);
    let correction = decoder
        .decode(&syndrome, DecoderRule::LogRatio, 10, Some(0.03), None).unwrap();

    assert_eq!(correction, exp_correction);
}

#[test]
fn min_sum_decode_96_3() {
    let tanner = alist_to_tanner(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/testfiles/96-3-963"
    ))
    .expect("96-3-963 should parse");
    assert_eq!((tanner.n, tanner.m), (96, 48));

    // syndrome
    let syndrome: Vec<Bit> = "000010000000000001011000000000000001010010000000"
        .bytes()
        .map(|bit| match bit {
            b'0' => Bit::ZERO,
            b'1' => Bit::ONE,
            _ => panic!("syndrome must contain only binary digits"),
        })
        .collect();

    // expected correction
    let mut exp_correction = vec![Bit::ZERO; tanner.n];
    exp_correction[26] = Bit::ONE;
    exp_correction[82] = Bit::ONE;
    exp_correction[87] = Bit::ONE;


    let decoder = Decoder::new(tanner);
    let correction = decoder
        .decode(&syndrome, DecoderRule::MinSum, 10, Some(0.03), None).unwrap();

    assert_eq!(correction, exp_correction);
}

#[test]
fn normalized_min_sum_decode_96_3() {
    let tanner = alist_to_tanner(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/testfiles/96-3-963"
    ))
    .expect("96-3-963 should parse");
    assert_eq!((tanner.n, tanner.m), (96, 48));

    // syndrome
    let syndrome: Vec<Bit> = "000010000000000001011000000000000001010010000000"
        .bytes()
        .map(|bit| match bit {
            b'0' => Bit::ZERO,
            b'1' => Bit::ONE,
            _ => panic!("syndrome must contain only binary digits"),
        })
        .collect();

    // expected correction
    let mut exp_correction = vec![Bit::ZERO; tanner.n];
    exp_correction[26] = Bit::ONE;
    exp_correction[82] = Bit::ONE;
    exp_correction[87] = Bit::ONE;


    let decoder = Decoder::new(tanner);
    let correction = decoder
        .decode(&syndrome, DecoderRule::NormalizedMinSum, 10, Some(0.03), None).unwrap();

    assert_eq!(correction, exp_correction);
}

#[test]
fn quantum_decode_logratio_bb72() {
    let tanner_x = alist_to_tanner(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/testfiles/bb72_Hx.alist"
    ))
    .expect("bb72-x should parse");
    assert_eq!((tanner_x.n, tanner_x.m), (72, 36));

    let tanner_z = alist_to_tanner(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/testfiles/bb72_Hz.alist"
    ))
    .expect("bb72-z should parse");
    assert_eq!((tanner_z.n, tanner_z.m), (72, 36));

    // syndrome
    let mut syndrome_x = vec![Bit::ZERO; tanner_x.m];
    let mut syndrome_z = vec![Bit::ZERO; tanner_z.m];
    syndrome_x[5] = Bit::ONE;
    syndrome_x[9] = Bit::ONE;
    syndrome_x[15] = Bit::ONE;
    syndrome_x[22] = Bit::ONE;
    syndrome_z[8] = Bit::ONE;
    syndrome_z[15] = Bit::ONE;
    syndrome_z[18] = Bit::ONE;
    syndrome_z[28] = Bit::ONE;

    // expected correction
    let mut exp_correction_x = vec![Bit::ZERO; tanner_x.n];
    let mut exp_correction_z = vec![Bit::ZERO; tanner_z.n];
    exp_correction_x[3] = Bit::ONE;
    exp_correction_x[40] = Bit::ONE;
    exp_correction_z[10] = Bit::ONE;
    exp_correction_z[57] = Bit::ONE;


    let decoder = QuantumDecoder::new(tanner_x, tanner_z);
    let (cor_x, cor_z) = decoder
        .decode(&syndrome_x,&syndrome_z,DecoderRule::LogRatio, 50, Some(0.01), None).unwrap();

    assert_eq!(cor_x, exp_correction_x);
    assert_eq!(cor_z, exp_correction_z);
}