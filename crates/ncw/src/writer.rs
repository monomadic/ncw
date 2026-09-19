//! PCM writing and encoding-preserving reconstruction.
use crate::bits::packed_values;
use crate::block::read_block;
use crate::{BlockHeader, NcwError, NcwHeader};
use std::io::{Cursor, Write};

type Result<T> = std::result::Result<T, NcwError>;
fn invalid(s: &'static str) -> NcwError {
    NcwError::InvalidHeader(s)
}

/// Stereo transform selection for newly encoded blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StereoMode {
    /// Pick mid/side only if exactly representable and smaller than direct encoding.
    Auto,
    Direct,
    /// Fail rather than round when left/right samples have different parity.
    MidSide,
}

/// Parameters for integer PCM writing. The initial writer supports mono/stereo 16/24-bit PCM.
#[derive(Clone, Copy, Debug)]
pub struct PcmSpec {
    pub channels: u16,
    pub bits_per_sample: u16,
    pub sample_rate: u32,
}

fn validate(samples: &[i32], spec: PcmSpec) -> Result<usize> {
    if !matches!(spec.channels, 1 | 2)
        || !matches!(spec.bits_per_sample, 16 | 24)
        || spec.sample_rate == 0
    {
        return Err(invalid(
            "writer requires mono/stereo PCM16/24 with nonzero sample rate",
        ));
    }
    if samples.len() % spec.channels as usize != 0 {
        return Err(invalid("incomplete sample frame"));
    }
    let limit = 1i32 << (spec.bits_per_sample - 1);
    if samples.iter().any(|&v| v < -limit || v >= limit) {
        return Err(invalid("PCM value out of range"));
    }
    let frames = samples.len() / spec.channels as usize;
    if frames > u32::MAX as usize {
        return Err(invalid("too many frames"));
    }
    Ok(frames)
}

fn pack(values: &[i32], bits: usize) -> Result<Vec<u8>> {
    if !(1..=32).contains(&bits) {
        return Err(invalid("invalid packed width"));
    }
    let mut out = Vec::new();
    let mut acc = 0u64;
    let mut available = 0;
    let mask = (1u64 << bits) - 1;
    for &v in values {
        if (v as i64) < -(1i64 << (bits - 1)) || (v as i64) >= (1i64 << (bits - 1)) {
            return Err(invalid("sample or delta does not fit block width"));
        }
        acc |= (v as u32 as u64 & mask) << available;
        available += bits;
        while available >= 8 {
            out.push(acc as u8);
            acc >>= 8;
            available -= 8;
        }
    }
    if available != 0 {
        out.push(acc as u8);
    }
    Ok(out)
}
// Width 1 is deliberately excluded: Kontakt decoding differed on a natural PCM16
// probe. Width 2 restored exact PCM; width-1 semantics are still unresolved.
fn width(values: &[i32]) -> usize {
    (2..=32)
        .find(|&bits| {
            values
                .iter()
                .all(|&v| (v as i64) >= -(1i64 << (bits - 1)) && (v as i64) < (1i64 << (bits - 1)))
        })
        .unwrap()
}
fn deltas(values: &[i32], terminal: i32) -> Vec<i32> {
    values
        .windows(2)
        .map(|v| v[1].wrapping_sub(v[0]))
        .chain(std::iter::once(terminal))
        .collect()
}
fn transform(channels: &mut [Vec<i32>]) -> Result<()> {
    if channels.len() != 2 {
        return Err(invalid("mid/side requires stereo"));
    }
    let (left, right) = channels.split_at_mut(1);
    for (l, r) in left[0].iter_mut().zip(&mut right[0]) {
        let (sum, diff) = (*l as i64 + *r as i64, *l as i64 - *r as i64);
        if sum & 1 != 0 {
            return Err(invalid(
                "mid/side cannot exactly encode opposite-parity channels",
            ));
        }
        *l = (sum / 2) as i32;
        *r = (diff / 2) as i32;
    }
    Ok(())
}
fn channels(samples: &[i32], count: usize) -> Vec<Vec<i32>> {
    (0..count)
        .map(|c| samples.iter().skip(c).step_by(count).copied().collect())
        .collect()
}
fn channel_block(
    values: &[i32],
    bits: i16,
    flags: u16,
    base: i32,
    padding: u32,
    depth: u16,
    terminal: i32,
) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    out.extend(0x3e9a0c16u32.to_le_bytes());
    out.extend(base.to_le_bytes());
    out.extend(bits.to_le_bytes());
    out.extend(flags.to_le_bytes());
    out.extend(padding.to_le_bytes());
    let packed = if bits > 0 {
        pack(&deltas(values, terminal), bits as usize)?
    } else {
        pack(
            values,
            if bits == 0 {
                depth as usize
            } else {
                bits.unsigned_abs() as usize
            },
        )?
    };
    out.extend(packed);
    Ok(out)
}
fn assemble(
    mut prefix: Vec<u8>,
    groups: Vec<Vec<u8>>,
    suffix: &[u8],
    spec: PcmSpec,
    frames: usize,
) -> Result<Vec<u8>> {
    let table = u32::try_from(prefix.len()).map_err(|_| invalid("header too large"))?;
    let start = table
        .checked_add(u32::try_from((groups.len() + 1) * 4).map_err(|_| invalid("table too large"))?)
        .ok_or(invalid("file too large"))?;
    let mut data = Vec::new();
    let mut offsets = vec![0u32];
    for group in groups {
        data.extend(group);
        offsets.push(u32::try_from(data.len()).map_err(|_| invalid("data too large"))?);
    }
    prefix[8..10].copy_from_slice(&spec.channels.to_le_bytes());
    prefix[10..12].copy_from_slice(&spec.bits_per_sample.to_le_bytes());
    prefix[12..16].copy_from_slice(&spec.sample_rate.to_le_bytes());
    prefix[16..20].copy_from_slice(&(frames as u32).to_le_bytes());
    prefix[20..24].copy_from_slice(&table.to_le_bytes());
    prefix[24..28].copy_from_slice(&start.to_le_bytes());
    prefix[28..32].copy_from_slice(&(data.len() as u32).to_le_bytes());
    for offset in offsets {
        prefix.extend(offset.to_le_bytes());
    }
    prefix.extend(data);
    prefix.extend(suffix);
    Ok(prefix)
}

/// Encode interleaved PCM to a new NCW. Unknown main-header bytes are initialized to zero.
/// This is a deterministic encoder, not a reproduction of NI's block selection policy.
pub fn encode_pcm(samples: &[i32], spec: PcmSpec, mode: StereoMode) -> Result<Vec<u8>> {
    let frames = validate(samples, spec)?;
    if mode == StereoMode::MidSide && spec.channels != 2 {
        return Err(invalid("mid/side requires stereo"));
    }
    let mut groups = Vec::new();
    for chunk in samples.chunks(512 * spec.channels as usize) {
        let mut direct = channels(chunk, spec.channels as usize);
        for c in &mut direct {
            c.resize(512, *c.last().unwrap());
        }
        let encode = |cs: &[Vec<i32>], flag: u16| -> Result<Vec<u8>> {
            let mut group = Vec::new();
            for c in cs {
                let w = width(&deltas(c, 0));
                let bits = if w < spec.bits_per_sample as usize {
                    w as i16
                } else {
                    -(spec.bits_per_sample as i16)
                };
                group.extend(channel_block(
                    c,
                    bits,
                    flag,
                    c[0],
                    0,
                    spec.bits_per_sample,
                    0,
                )?);
            }
            Ok(group)
        };
        let mut group = encode(&direct, 0)?;
        if mode != StereoMode::Direct && spec.channels == 2 {
            let mut ms = direct;
            match transform(&mut ms) {
                Ok(()) => {
                    let candidate = encode(&ms, 1)?;
                    if mode == StereoMode::MidSide || candidate.len() < group.len() {
                        group = candidate;
                    }
                }
                Err(e) if mode == StereoMode::MidSide => return Err(e),
                Err(_) => {}
            }
        }
        groups.push(group);
    }
    let mut prefix = vec![0; 120];
    prefix[..8].copy_from_slice(&0x01A89ED631010000u64.to_be_bytes());
    assemble(prefix, groups, &[], spec, frames)
}

/// Re-encode PCM using an existing NCW's encoding choices. The template supplies opaque
/// headers, block widths/flags, unused terminal deltas and padded tail samples. Active
/// audio payloads are reconstructed from `samples`, never copied. Rejects unsupported
/// flags, malformed framing and values that do not fit the template. Supports raw and
/// delta blocks for mono/stereo PCM16/24. Does not clip, round or normalize samples.
pub fn encode_pcm_with_template(
    samples: &[i32],
    spec: PcmSpec,
    template: &[u8],
) -> Result<Vec<u8>> {
    let frames = validate(samples, spec)?;
    let header = NcwHeader::read(&mut Cursor::new(template))?;
    if header.channels != spec.channels
        || header.bits_per_sample != spec.bits_per_sample
        || header.sample_rate != spec.sample_rate
        || header.num_samples as usize != frames
    {
        return Err(invalid("template audio parameters differ"));
    }
    let table = header.blocks_offset as usize;
    let start = header.data_offset as usize;
    let end = start
        .checked_add(header.data_size as usize)
        .ok_or(invalid("data overflow"))?;
    let count = frames.div_ceil(512);
    if table < 120 || start < table || end > template.len() || start - table != (count + 1) * 4 {
        return Err(invalid("invalid template table/data bounds"));
    }
    let offsets: Vec<usize> = template[table..start]
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()) as usize)
        .collect();
    if offsets[0] != 0
        || offsets[count] != header.data_size as usize
        || offsets.windows(2).any(|v| v[0] >= v[1])
    {
        return Err(invalid("invalid terminal block offsets"));
    }
    let mut groups = Vec::new();
    for (index, offsets) in offsets.windows(2).enumerate() {
        let group = &template[start + offsets[0]..start + offsets[1]];
        let mut cursor = Cursor::new(group);
        let first_flag = group.get(10..12).ok_or(invalid("truncated block"))?;
        let mid_side = u16::from_le_bytes(first_flag.try_into().unwrap()) & 1 != 0;
        let begin = index * 512 * spec.channels as usize;
        let stop = samples.len().min(begin + 512 * spec.channels as usize);
        let mut cs = channels(&samples[begin..stop], spec.channels as usize);
        if mid_side {
            transform(&mut cs)?;
        }
        let mut result = Vec::new();
        for c in &mut cs {
            let pos = cursor.position() as usize;
            let bh = BlockHeader::read(&mut cursor)?;
            if matches!(bh.bits, 0 | 1) {
                return Err(invalid(
                    "zero/one-bit semantics are not validated for template writing",
                ));
            }
            if bh.flags > 1 {
                return Err(invalid("template writer supports only PCM flags 0/1"));
            }
            let padding = u32::from_le_bytes(group[pos + 12..pos + 16].try_into().unwrap());
            let body = cursor.position() as usize;
            let mut original = Vec::new();
            read_block(&mut cursor, &header, &bh, &mut original)?;
            let tail = c.len();
            c.extend_from_slice(&original[tail..]);
            let terminal = if bh.bits > 0 {
                packed_values(&group[body..cursor.position() as usize], bh.bits as usize)
                    .last()
                    .unwrap()
            } else {
                0
            };
            result.extend(channel_block(
                c,
                bh.bits,
                bh.flags,
                if bh.bits > 0 { c[0] } else { bh.base_value },
                padding,
                spec.bits_per_sample,
                terminal,
            )?);
        }
        if cursor.position() as usize != group.len() {
            return Err(invalid("unconsumed template block bytes"));
        }
        groups.push(result);
    }
    assemble(
        template[..table].to_vec(),
        groups,
        &template[end..],
        spec,
        frames,
    )
}

/// Write encoded bytes to any output stream.
pub fn write_pcm<W: Write>(
    writer: &mut W,
    samples: &[i32],
    spec: PcmSpec,
    mode: StereoMode,
) -> Result<()> {
    writer.write_all(&encode_pcm(samples, spec, mode)?)?;
    Ok(())
}
