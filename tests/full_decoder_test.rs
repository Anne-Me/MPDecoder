use MPDecoder::binary_decoding::{Decoder, DecoderRule};
use MPDecoder::parsing::alist_to_tanner;
use MPDecoder::tanner_graph::Bit;

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
