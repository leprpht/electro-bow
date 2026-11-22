use super::Attack;
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
    Attack::new(48000.0).process(&mut actual, 2, 2048, 0.0);
    assert_eq!(actual, original);
}

#[test]
fn finite_controls_are_clamped_to_supported_range() {
    for (requested, clamped) in [(-10.0, 0.0), (500.0 * 2.0, 500.0)] {
        let mut actual = fixture();
        let mut expected = actual.clone();
        Attack::new(48000.0).process(&mut actual, 2, 2048, requested);
        Attack::new(48000.0).process(&mut expected, 2, 2048, clamped);
        assert_eq!(actual, expected);
    }
}

#[test]
fn reset_after_control_changes_matches_fresh_processor() {
    let mut processor = Attack::new(48000.0);
    // Exercise coefficient/control history at several settings before reset.
    for amount in [500.0, 500.0 * 0.25, 500.0 * 0.75] {
        processor.process(&mut fixture(), 2, 2048, amount);
    }
    processor.reset();
    let mut actual = fixture();
    let mut expected = actual.clone();
    processor.process(&mut actual, 2, 2048, 500.0 * 0.75);
    Attack::new(48000.0).process(&mut expected, 2, 2048, 500.0 * 0.75);
    assert_eq!(actual, expected);
}

#[test]
fn processing_respects_sample_count_and_configured_channels() {
    let original = fixture();
    let mut actual = original.clone();
    Attack::new(48000.0).process(&mut actual, 1, 1024, 500.0);
    for (processed, source) in actual.iter().zip(&original) {
        assert_eq!(processed[1], source[1], "inactive channel");
        assert_eq!(processed[0][1024..], source[0][1024..], "unused tail");
    }
    assert_ne!(
        actual[0][0][..1024],
        original[0][0][..1024],
        "the fundamental must swell too"
    );
    assert_ne!(actual[3][0][..1024], original[3][0][..1024]);
}

#[test]
fn stereo_link_preserves_opposite_polarity_channel_ratio() {
    let mut actual = fixture();
    Attack::new(48000.0).process(&mut actual, 2, 2048, 500.0);
    for band in actual {
        for (left, right) in band[0].iter().zip(&band[1]) {
            assert!((left * -0.5 - right).abs() < 1e-7);
        }
    }
}

#[test]
fn silence_stays_silent_after_signal_history() {
    let mut processor = Attack::new(48000.0);
    processor.process(&mut fixture(), 2, 2048, 500.0);
    let mut silence = std::array::from_fn(|_| std::array::from_fn(|_| vec![0.0; 2048]));
    processor.process(&mut silence, 2, 2048, 500.0);
    assert!(silence.iter().flatten().flatten().all(|&x| x == 0.0));
}

#[test]
fn automating_attack_in_silence_engages_the_next_swell_and_releases_smoothly() {
    let mut attack = Attack::new(48000.0);
    let mut bands: [[Vec<f32>; 2]; 4] = Default::default();
    for band in &mut bands {
        band[0].resize(4096, 0.0);
    }
    attack.process(&mut bands, 1, 480, 0.0);
    attack.process(&mut bands, 1, 480, 500.0);
    bands[0][0].fill(0.5);
    attack.process(&mut bands, 1, 480, 500.0);
    let mut previous = bands[0][0][479];
    assert!(previous < 0.05, "long swell was still bypassed");
    bands[0][0].fill(0.5);
    attack.process(&mut bands, 1, 4096, 0.0);
    for &current in &bands[0][0] {
        assert!((current - previous).abs() < 0.002, "abrupt gain change");
        previous = current;
    }
    assert!(
        (previous - 0.5).abs() < 1e-6,
        "zero attack must recover unity"
    );
}

fn shaped(signal: &[f32], amount: f64, scale: f32, chunk: usize) -> Vec<f32> {
    let mut bank = FilterBank::new(48000.0, chunk, 1).unwrap();
    let mut result = Vec::new();
    for input in signal.chunks(chunk) {
        let input: Vec<_> = input.iter().map(|x| x * scale).collect();
        bank.split.split_channel(0, &input);
        bank.attack
            .process(&mut bank.split.bands, 1, input.len(), amount);
        let mut output = vec![0.0; input.len()];
        bank.split.recombine_channel(0, &mut output);
        result.extend(output);
    }
    result
}

#[test]
fn attack_preserves_level_scaling_and_continuous_playing() {
    for kind in 0..5 {
        let signal: Vec<f32> = (0..48000)
            .map(|i| {
                let t = i as f64 / 48000.0;
                let carrier = (t * 220.0 * std::f64::consts::TAU).sin()
                    + 0.4 * (t * 330.0 * std::f64::consts::TAU).sin()
                    + 0.2 * (t * 6000.0 * std::f64::consts::TAU).sin();
                let envelope = match kind {
                    0 => (-t * 3.0).exp(),                                     // single pluck
                    1 => 1.0,                                                  // sustained chord
                    2 => (-(t % 0.06) * 50.0).exp(),                           // fast picking
                    3 => 0.6 + 0.4 * (t * 12.0 * std::f64::consts::TAU).sin(), // tremolo
                    _ => {
                        if i < 24000 {
                            0.1
                        } else {
                            1.0
                        }
                    } // changing playing level
                };
                (0.3 * carrier * envelope) as f32
            })
            .collect();
        for amount in [0.0, 250.0, 500.0] {
            let loud = shaped(&signal, amount, 1.0, 257);
            let quiet = shaped(&signal, amount, 0.1, 257);
            let partitioned = shaped(&signal, amount, 1.0, 17);
            for ((a, b), c) in loud.iter().zip(quiet).zip(partitioned) {
                assert!(a.is_finite());
                assert!((a * 0.1 - b).abs() < 1e-7, "playing level scaling");
                assert!((a - c).abs() < 1e-7, "block invariance");
            }
        }
    }
}

#[test]
fn attack_softens_guitar_fundamentals_and_pick_noise_and_recovers_sustain() {
    for frequency in [82.4, 330.0, 6000.0] {
        let signal: Vec<f32> = (0..96000)
            .map(|i| (i as f64 * frequency * std::f64::consts::TAU / 48000.0).sin() as f32)
            .collect();
        let energy = |x: &[f32]| x.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
        let dry = shaped(&signal, 0.0, 1.0, 64);
        let middle = shaped(&signal, 250.0, 1.0, 64);
        let smooth = shaped(&signal, 500.0, 1.0, 64);
        let ratio = energy(&smooth[48..960]) / energy(&dry[48..960]);
        assert!(ratio < 0.025);
        assert!(energy(&smooth[48..960]) < energy(&middle[48..960]));
        assert!(energy(&middle[48..960]) < energy(&dry[48..960]));
        let sustain = energy(&smooth[80000..]) / energy(&dry[80000..]);
        assert!(
            (sustain - 1.0).abs() < 0.04,
            "{frequency} Hz steady-state power ratio {sustain}"
        );
        println!("Attack onset energy ratio={ratio:.6}, sustained energy ratio={sustain:.6}");
    }
}

#[test]
fn attack_softens_a_new_pick_above_a_ringing_string() {
    let rate = 48000.0;
    let partial = |hz: f64, i: usize| (std::f64::consts::TAU * hz * i as f64 / rate).sin();
    let input: Vec<_> = (0..48000)
        .map(|i| {
            let pick = if i >= 24000 {
                0.14 * partial(6000.0, i)
            } else {
                0.0
            };
            (0.35 * partial(100.0, i) + pick) as f32
        })
        .collect();
    let audio = shaped(&input, 120.0, 1.0, 97);
    // Use complete cycles to isolate the two components in the same 10 ms.
    let amplitude = |start: usize, hz: f64| {
        let (mut re, mut im) = (0.0, 0.0);
        for (i, &sample) in audio[start..start + 480].iter().enumerate() {
            let phase = std::f64::consts::TAU * hz * i as f64 / rate;
            re += f64::from(sample) * phase.cos();
            im += f64::from(sample) * phase.sin();
        }
        re.hypot(im) / 240.0
    };
    assert!(
        amplitude(24000, 6000.0) < 0.08,
        "treble pick breaks through"
    );
    assert!(amplitude(24000, 100.0) > 0.32, "ringing bass ducks");
    assert!(
        amplitude(43200, 6000.0) > 0.135,
        "sustained brightness is lost"
    );
}
