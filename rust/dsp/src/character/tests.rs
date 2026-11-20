use super::Character;

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
