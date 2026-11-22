use super::Character;
use crate::filter_bank::FilterBank;

fn fixture() -> [[Vec<f32>; 2]; 4] {
    std::array::from_fn(|_| {
        std::array::from_fn(|channel| {
            (0..2048)
                .map(|i| {
                    let sample = if i % 257 < 32 { 0.8 } else { 0.1 };
                    if channel == 0 { sample } else { -sample * 0.5 }
                })
                .collect()
        })
    })
}

#[test]
fn zero_amount_is_exact_bypass() {
    let original = fixture();
    let mut actual = original.clone();
    Character::new(48000.0).process(&mut actual, 2, 2048, 0.0);
    assert_eq!(actual, original);
}

#[test]
fn finite_controls_are_clamped_to_supported_range() {
    for (requested, clamped) in [(-10.0, 0.0), (1.0 * 2.0, 1.0)] {
        let mut actual = fixture();
        let mut expected = actual.clone();
        Character::new(48000.0).process(&mut actual, 2, 2048, requested);
        Character::new(48000.0).process(&mut expected, 2, 2048, clamped);
        assert_eq!(actual, expected);
    }
}

#[test]
fn reset_after_control_changes_matches_fresh_processor() {
    let mut processor = Character::new(48000.0);
    // Exercise coefficient/control history at several settings before reset.
    for amount in [1.0, 1.0 * 0.25, 1.0 * 0.75] {
        processor.process(&mut fixture(), 2, 2048, amount);
    }
    processor.reset();
    let mut actual = fixture();
    let mut expected = actual.clone();
    processor.process(&mut actual, 2, 2048, 1.0 * 0.75);
    Character::new(48000.0).process(&mut expected, 2, 2048, 1.0 * 0.75);
    assert_eq!(actual, expected);
}

#[test]
fn processing_respects_sample_count_and_configured_channels() {
    let original = fixture();
    let mut actual = original.clone();
    Character::new(48000.0).process(&mut actual, 1, 1024, 1.0);
    for (processed, source) in actual.iter().zip(&original) {
        assert_eq!(processed[1], source[1], "inactive channel");
        assert_eq!(processed[0][1024..], source[0][1024..], "unused tail");
    }
    assert_eq!(actual[0], original[0], "lowest band must remain unchanged");
    assert_ne!(actual[3][0][..1024], original[3][0][..1024]);
}

#[test]
fn character_preserves_off_centre_stereo_balance() {
    let mut actual = fixture();
    for band in &mut actual {
        band[1] = band[0].iter().map(|x| -0.25 * x).collect();
    }
    Character::new(48000.0).process(&mut actual, 2, 2048, 1.0);
    for band in actual {
        for (left, right) in band[0].iter().zip(&band[1]) {
            assert!((0.25 * left + right).abs() < 1e-7);
        }
    }
}

#[test]
fn silence_stays_silent_after_signal_history() {
    let mut processor = Character::new(48000.0);
    processor.process(&mut fixture(), 2, 2048, 1.0);
    let mut silence = std::array::from_fn(|_| std::array::from_fn(|_| vec![0.0; 2048]));
    processor.process(&mut silence, 2, 2048, 1.0);
    assert!(silence.iter().flatten().flatten().all(|&x| x == 0.0));
}

fn character_render(input: &[f32], amount: f64, scale: f32, chunk: usize, rate: f64) -> Vec<f32> {
    let mut bank = FilterBank::new(rate, chunk, 1).unwrap();
    let mut result = Vec::new();
    for source in input.chunks(chunk) {
        let source: Vec<_> = source.iter().map(|x| x * scale).collect();
        bank.split.split_channel(0, &source);
        bank.character
            .process(&mut bank.split.bands, 1, source.len(), amount);
        let mut output = vec![0.0; source.len()];
        bank.split.recombine_channel(0, &mut output);
        result.extend(output);
    }
    result
}

#[test]
fn character_chords_fast_playing_scaling_and_partitioning() {
    for rate in [8000.0, 44100.0, 48000.0, 96000.0, 192000.0] {
        let source: Vec<f32> = (0..12000)
            .map(|i| {
                let t = i as f64 / rate;
                let envelope = (-40.0 * (t % 0.06)).exp();
                (envelope
                    * [147.0, 220.0, 294.0, 1100.0]
                        .iter()
                        .map(|f| 0.15 * (std::f64::consts::TAU * f * t).sin())
                        .sum::<f64>()) as f32
            })
            .collect();
        for amount in [0.0, 0.5, 1.0] {
            let a = character_render(&source, amount, 1.0, 256, rate);
            let b = character_render(&source, amount, 0.1, 17, rate);
            let c = character_render(&source, amount, 1.0, 17, rate);
            for ((x, y), z) in a.iter().zip(b).zip(c) {
                assert!(x.is_finite() && x.abs() < 1.0);
                assert!((x * 0.1 - y).abs() < 1e-7);
                assert!((x - z).abs() < 1e-7);
            }
            if amount == 0.0 {
                assert!(a.iter().zip(&source).all(|(a, b)| (a - b).abs() < 2e-7));
            }
        }
    }
}

#[test]
fn character_silence_stereo_and_reset() {
    let mut bank = FilterBank::new(48000.0, 256, 2).unwrap();
    for amount in [0.0, 0.5, 1.0] {
        for _ in 0..10 {
            bank.character
                .process(&mut bank.split.bands, 2, 256, amount);
            assert!(
                bank.split
                    .bands
                    .iter()
                    .flatten()
                    .flatten()
                    .all(|x| *x == 0.0)
            );
        }
    }
    let signal: Vec<f32> = (0..256).map(|i| (i as f32 * 0.17).sin() * 0.5).collect();
    bank.split.split_channel(0, &signal);
    bank.split.split_channel(1, &signal);
    bank.character.process(&mut bank.split.bands, 2, 256, 1.0);
    for band in &bank.split.bands {
        assert_eq!(band[0], band[1]);
    }
    let first = bank.split.bands.clone();
    bank.reset();
    bank.split.split_channel(0, &signal);
    bank.split.split_channel(1, &signal);
    bank.character.process(&mut bank.split.bands, 2, 256, 1.0);
    // Reset restores the envelope and control smoother.
    let mut fresh = FilterBank::new(48000.0, 256, 2).unwrap();
    fresh.split.split_channel(0, &signal);
    fresh.split.split_channel(1, &signal);
    fresh.character.process(&mut fresh.split.bands, 2, 256, 1.0);
    assert_eq!(bank.split.bands, fresh.split.bands);
    assert_ne!(first, bank.split.bands);
}

#[test]
fn character_two_tone_artifact_budget() {
    let rate = 48000.0;
    let source: Vec<f32> = (0..96000)
        .map(|i| {
            let t = i as f64 / rate;
            (0.2 * (std::f64::consts::TAU * 900.0 * t).sin()
                + 0.2 * (std::f64::consts::TAU * 1300.0 * t).sin()) as f32
        })
        .collect();
    let output = character_render(&source, 1.0, 1.0, 256, rate);
    let tail = &output[48000..];
    let total = tail.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>() / rate;
    let mut fundamental = 0.0;
    for frequency in [900.0, 1300.0] {
        let mut re = 0.0;
        let mut im = 0.0;
        for (i, &x) in tail.iter().enumerate() {
            let phase = std::f64::consts::TAU * frequency * i as f64 / rate;
            re += f64::from(x) * phase.cos();
            im += f64::from(x) * phase.sin();
        }
        fundamental += 2.0 * (re * re + im * im) / (rate * rate);
    }
    let residual = ((total - fundamental).max(0.0) / total).sqrt();
    println!(
        "Character two-tone residual (includes intended texture): {:.3} dBr",
        20.0 * residual.log10()
    );
    assert!(residual < 0.05, "excessive non-fundamental energy");
}
