use ncw::{NcwReader, PcmSpec, StereoMode, encode_pcm, encode_pcm_with_template};
use std::io::Cursor;
fn spec(depth: u16) -> PcmSpec {
    PcmSpec {
        channels: 2,
        bits_per_sample: depth,
        sample_rate: 48000,
    }
}
fn decode(data: &[u8]) -> Vec<i32> {
    NcwReader::read(Cursor::new(data))
        .unwrap()
        .decode_samples()
        .unwrap()
}
#[test]
fn fresh_pcm_extremes_partial_blocks_and_modes() {
    for depth in [16, 24] {
        for frames in [0, 1, 511, 512, 513] {
            for mode in [StereoMode::Direct, StereoMode::Auto, StereoMode::MidSide] {
                let limit = 1 << (depth - 1);
                let pcm: Vec<i32> = (0..frames)
                    .flat_map(|i| {
                        let v = if i % 2 == 0 { -limit } else { limit - 1 };
                        [v, v]
                    })
                    .collect();
                let bytes = encode_pcm(&pcm, spec(depth), mode).unwrap();
                assert_eq!(decode(&bytes), pcm);
                assert_eq!(
                    encode_pcm_with_template(&pcm, spec(depth), &bytes).unwrap(),
                    bytes
                );
            }
        }
    }
}
#[test]
fn parity_is_never_silently_rounded() {
    let pcm = vec![3, 0, 4, 1];
    assert!(encode_pcm(&pcm, spec(16), StereoMode::MidSide).is_err());
    assert_eq!(
        decode(&encode_pcm(&pcm, spec(16), StereoMode::Auto).unwrap()),
        pcm
    );
}
#[test]
fn template_rebuilds_payload_instead_of_copying_it() {
    let pcm: Vec<i32> = (0..513).flat_map(|i| [i * 2, i * 2]).collect();
    let mut original = encode_pcm(&pcm, spec(16), StereoMode::MidSide).unwrap();
    original[40] = 0xaa; // opaque header survives
    original.extend([1, 2, 3]); // trailing bytes survive
    let mut changed = pcm.clone();
    changed[0] += 2;
    changed[1] += 2;
    let output = encode_pcm_with_template(&changed, spec(16), &original).unwrap();
    assert_ne!(output, original);
    assert_eq!(decode(&output), changed);
    assert_eq!(output[40], 0xaa);
    assert!(output.ends_with(&[1, 2, 3]));
    changed[0] = 32767;
    assert!(encode_pcm_with_template(&changed, spec(16), &original).is_err());
}
#[test]
fn shipped_pcm_fixtures_are_byte_identical() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    for name in [
        "16-bit-mono",
        "16-bit-stereo",
        "24-bit-mono",
        "24-bit-stereo",
        "testfile-onezero-16-bit-stereo",
        "testfile-onezero-16-bit-stereo-multiblock",
    ] {
        let bytes = std::fs::read(dir.join(format!("{name}.ncw"))).unwrap();
        let mut reader = NcwReader::read(Cursor::new(&bytes)).unwrap();
        let s = PcmSpec {
            channels: reader.header.channels,
            bits_per_sample: reader.header.bits_per_sample,
            sample_rate: reader.header.sample_rate,
        };
        let pcm = reader.decode_samples().unwrap();
        assert_eq!(
            encode_pcm_with_template(&pcm, s, &bytes).unwrap(),
            bytes,
            "{name}"
        );
    }
}
#[test]
fn first_channel_controls_stereo_transform() {
    let pcm = vec![12, 8];
    let bytes = encode_pcm(&pcm, spec(16), StereoMode::MidSide).unwrap();
    let start = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
    let bits = i16::from_le_bytes(bytes[start + 8..start + 10].try_into().unwrap());
    let second = start + 16 + bits as usize * 64;
    let mut first_only = bytes.clone();
    first_only[second + 10] = 0;
    assert_eq!(decode(&first_only), pcm);
    let mut second_only = bytes;
    second_only[start + 10] = 0;
    assert_eq!(decode(&second_only), vec![10, 2]);
}
#[test]
fn malformed_templates_rejected_without_panics() {
    let pcm = vec![2, 2];
    let bytes = encode_pcm(&pcm, spec(16), StereoMode::Auto).unwrap();
    for cut in 0..bytes.len() {
        assert!(encode_pcm_with_template(&pcm, spec(16), &bytes[..cut]).is_err());
    }
    let mut bad = bytes.clone();
    bad[124..128].copy_from_slice(&0u32.to_le_bytes());
    assert!(encode_pcm_with_template(&pcm, spec(16), &bad).is_err());
    let mut bad = bytes;
    bad[138] = 2;
    assert!(encode_pcm_with_template(&pcm, spec(16), &bad).is_err());
}

#[test]
fn one_bit_delta_encoding_is_avoided() {
    let pcm = vec![0, 0, -1, -1];
    let bytes = encode_pcm(&pcm, spec(16), StereoMode::Auto).unwrap();
    let start = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
    assert_eq!(
        i16::from_le_bytes(bytes[start + 8..start + 10].try_into().unwrap()),
        2
    );
}

#[test]
fn raw_fallback_uses_kontakt_negative_full_depth() {
    for depth in [16, 24] {
        let limit = 1 << (depth - 1);
        let pcm = vec![-limit, -limit, limit - 1, limit - 1];
        let bytes = encode_pcm(&pcm, spec(depth), StereoMode::Direct).unwrap();
        let start = u32::from_le_bytes(bytes[24..28].try_into().unwrap()) as usize;
        assert_eq!(
            i16::from_le_bytes(bytes[start + 8..start + 10].try_into().unwrap()),
            -(depth as i16)
        );
        assert_eq!(decode(&bytes), pcm);
        let mut zero_width = bytes.clone();
        zero_width[start + 8..start + 10].copy_from_slice(&0i16.to_le_bytes());
        assert!(encode_pcm_with_template(&pcm, spec(depth), &zero_width).is_err());
    }
}
