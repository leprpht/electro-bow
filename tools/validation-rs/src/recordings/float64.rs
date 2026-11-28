//! IEEE binary64 recording input, which hound's sample reader cannot represent.
use crate::{Result, ensure};
use std::{fs, path::Path};

pub(super) fn read(path: &Path) -> Result<(u32, Vec<f64>)> {
    let file = fs::read(path)?;
    let header = file.get(..12).ok_or("truncated WAV header")?;
    let big = &header[..4] == b"RIFX";
    ensure(
        (big || &header[..4] == b"RIFF") && &header[8..] == b"WAVE",
        "expected WAV container",
    )?;
    let u32_at = |word: &[u8]| {
        let bytes = word.try_into().unwrap();
        if big {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        }
    };
    let u16_at = |word: &[u8]| {
        let bytes = word.try_into().unwrap();
        if big {
            u16::from_be_bytes(bytes)
        } else {
            u16::from_le_bytes(bytes)
        }
    };
    let mut chunks = file
        .get(8..)
        .and_then(|b| b.get(..u32_at(&header[4..8]) as usize))
        .and_then(|b| b.get(4..))
        .ok_or("invalid RIFF length")?;
    let mut rate = None;
    let mut samples = None;
    while !chunks.is_empty() {
        let h = chunks.get(..8).ok_or("truncated WAV chunk header")?;
        let size = u32_at(&h[4..]) as usize;
        let body = chunks[8..].get(..size).ok_or("truncated WAV chunk")?;
        match &h[..4] {
            b"fmt " => {
                ensure(
                    rate.is_none() && body.len() >= 16,
                    "invalid WAV format header",
                )?;
                ensure(
                    body.len() == 16
                        || (body.len() >= 18
                            && usize::from(u16_at(&body[16..18])) <= body.len() - 18),
                    "truncated WAV format extension",
                )?;
                let tag = u16_at(&body[..2]);
                let extended = tag == 0xfffe
                    && !big
                    && body.len() >= 40
                    && u16_at(&body[16..18]) == 22
                    && matches!(u16_at(&body[18..20]), 0 | 64)
                    && body[24..40] == [3, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113];
                let hz = u32_at(&body[4..8]);
                ensure(
                    (tag == 3 || extended)
                        && u16_at(&body[2..4]) == 1
                        && hz >= 16000
                        && hz.checked_mul(8) == Some(u32_at(&body[8..12]))
                        && u16_at(&body[12..14]) == 8
                        && u16_at(&body[14..16]) == 64,
                    "unsupported binary64 WAV format",
                )?;
                rate = Some(hz);
            }
            b"data" => {
                ensure(
                    rate.is_some() && samples.is_none() && size.is_multiple_of(8),
                    "invalid binary64 WAV data",
                )?;
                let words = body
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|w| {
                        if big {
                            f64::from_be_bytes(*w)
                        } else {
                            f64::from_le_bytes(*w)
                        }
                    })
                    .collect::<Vec<_>>();
                ensure(
                    words.iter().all(|v| v.is_finite()),
                    "nonfinite decoded recording",
                )?;
                samples = Some(words);
            }
            _ => {}
        }
        chunks = chunks[8..]
            .get(size.next_multiple_of(2)..)
            .ok_or("missing WAV padding")?;
    }
    Ok((
        rate.ok_or("missing WAV format")?,
        samples.ok_or("missing WAV samples")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_binary64_words_and_rejects_damaged_payloads() {
        let words = [0.25_f64.next_up(), -0.0];
        let mut wav = b"RIFF".to_vec();
        wav.extend(52_u32.to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16_u32.to_le_bytes());
        wav.extend([3, 0, 1, 0]);
        wav.extend(48000_u32.to_le_bytes());
        wav.extend(384000_u32.to_le_bytes());
        wav.extend([8, 0, 64, 0]);
        wav.extend(b"data");
        wav.extend(16_u32.to_le_bytes());
        wav.extend(words.iter().flat_map(|w| w.to_le_bytes()));
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.wav");
        fs::write(&file, &wav).unwrap();
        let (rate, decoded) = super::super::read(&file).unwrap();
        assert_eq!(rate, 48000);
        assert_eq!(
            decoded.iter().map(|s| s.to_bits()).collect::<Vec<_>>(),
            words.map(f64::to_bits)
        );
        fs::write(&file, &wav[..wav.len() - 1]).unwrap();
        assert!(super::super::read(&file).is_err());
        wav[44..52].copy_from_slice(&f64::NAN.to_le_bytes());
        fs::write(&file, wav).unwrap();
        assert!(super::super::read(&file).is_err());
    }
}
