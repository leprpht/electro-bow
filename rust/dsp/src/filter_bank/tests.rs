use super::*;

#[test]
fn constructor_rejects_invalid_dimensions_and_rates() {
    for rate in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -48000.0] {
        assert!(FilterBank::new(rate, 64, 1).is_none());
    }
    for (capacity, channels) in [(0, 1), (64, 0), (64, 3), (64, usize::MAX)] {
        assert!(FilterBank::new(48000.0, capacity, channels).is_none());
    }
    for channels in [1, MAX_CHANNELS] {
        assert!(FilterBank::new(48000.0, 1, channels).is_some());
    }
}

#[test]
fn crossovers_keep_their_ratios_when_scaled_below_nyquist() {
    for rate in [1000.0, 4000.0, 8000.0, 48000.0, 192000.0] {
        let bank = FilterBank::new(rate, 8, 1).unwrap();
        let expected_high = 3000.0_f64.min(rate * 0.45);
        for (actual, nominal) in bank.crossovers_hz.iter().zip(CROSSOVERS_HZ) {
            assert!((actual - nominal / 3000.0 * expected_high).abs() < 1e-10);
            assert!(*actual < rate * 0.5);
        }
        assert!(bank.crossovers_hz.windows(2).all(|pair| pair[0] < pair[1]));
    }
}

#[test]
fn invalid_samples_match_explicitly_sanitized_input_and_history() {
    let input = [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        -f32::MAX,
        0.5,
    ];
    let sanitized = [0.0, 0.0, 0.0, 16.0, -16.0, 0.5];
    let mut actual = FilterBank::new(48000.0, input.len(), 1).unwrap();
    let mut expected = FilterBank::new(48000.0, input.len(), 1).unwrap();
    actual.split_channel(0, &input);
    expected.split_channel(0, &sanitized);
    assert_eq!(actual.bands, expected.bands);
    assert_eq!(actual.states, expected.states);
    let mut reconstructed = [0.0; 6];
    actual.recombine_channel(0, &mut reconstructed);
    for (actual, expected) in reconstructed.into_iter().zip(sanitized) {
        assert!((actual - expected).abs() < 2e-6);
    }
    actual.split_channel(0, &[0.25; 6]);
    expected.split_channel(0, &[0.25; 6]);
    assert_eq!(
        actual.bands, expected.bands,
        "faults must not poison history"
    );
}

#[test]
fn short_blocks_leave_unused_storage_and_other_channel_untouched() {
    let mut bank = FilterBank::new(48000.0, 8, 2).unwrap();
    for band in &mut bank.bands {
        for channel in band {
            channel.fill(123.0);
        }
    }
    bank.split_channel(0, &[1.0, -0.5]);
    for band in &bank.bands {
        assert_eq!(band[0][2..], [123.0; 6]);
        assert_eq!(band[1], [123.0; 8]);
    }
    assert_eq!(bank.states[1], [0.0; 3]);
    let mut output = [42.0; 8];
    bank.recombine_channel(0, &mut output[..2]);
    assert!((output[0] - 1.0).abs() < 1e-7);
    assert!((output[1] + 0.5).abs() < 1e-7);
    assert_eq!(output[2..], [42.0; 6]);
}

#[test]
fn reset_clears_storage_and_history_without_changing_preparation() {
    let mut bank = FilterBank::new(4000.0, 32, 2).unwrap();
    let crossovers = bank.crossovers_hz;
    for channel in 0..2 {
        bank.split_channel(channel, &[1.0; 32]);
    }
    bank.valid_samples = 32;
    bank.reset();
    assert_eq!(bank.valid_samples, 0);
    assert_eq!(bank.states, [[0.0; 3]; 2]);
    assert!(bank.bands.iter().flatten().flatten().all(|&x| x == 0.0));
    assert_eq!(bank.crossovers_hz, crossovers);
    assert_eq!((bank.capacity, bank.channels), (32, 2));
    let mut fresh = FilterBank::new(4000.0, 32, 2).unwrap();
    bank.split_channel(0, &[0.25; 32]);
    fresh.split_channel(0, &[0.25; 32]);
    assert_eq!(bank.bands, fresh.bands);
    assert_eq!(bank.states, fresh.states);
}
