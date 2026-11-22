use super::Output;

#[test]
fn signal_at_or_below_ceiling_is_unchanged() {
    let original = [0.0, 0.2, -0.5, 0.98, -0.98];
    let mut samples = original;
    Output::new(48000.0).process(&mut [&mut samples], 0.5);
    assert_eq!(samples, original);
}

#[test]
fn overload_in_either_channel_controls_both_channels() {
    for overload_on_left in [true, false] {
        let mut loud = [2.0, -4.0];
        let mut quiet = [0.5, -1.0];
        let mut output = Output::new(48000.0);
        if overload_on_left {
            output.process(&mut [&mut loud, &mut quiet], 0.5);
        } else {
            output.process(&mut [&mut quiet, &mut loud], 0.5);
        }
        for (actual, expected) in loud.into_iter().zip([0.98, -0.98]) {
            assert!((actual - expected).abs() < 1e-7);
        }
        for (actual, expected) in quiet.into_iter().zip([0.245, -0.245]) {
            assert!((actual - expected).abs() < 1e-7);
        }
    }
}

#[test]
fn non_finite_samples_are_silenced_without_muting_the_other_channel() {
    let mut invalid = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY];
    let mut finite = [0.25, -0.5, 0.75];
    Output::new(48000.0).process(&mut [&mut invalid, &mut finite], 0.5);
    assert_eq!(invalid, [0.0; 3]);
    assert_eq!(finite, [0.25, -0.5, 0.75]);
}

#[test]
fn release_recovers_over_time_and_reset_restores_unity() {
    for rate in [8000.0, 48000.0, 192000.0] {
        let mut output = Output::new(rate);
        output.process(&mut [&mut [4.0]], 0.5);
        let mut recovery = vec![0.5; (rate * 0.25) as usize];
        output.process(&mut [&mut recovery], 0.5);
        assert!(recovery[0] < 0.13, "gain must not recover instantly");
        assert!(recovery.windows(2).all(|pair| pair[0] <= pair[1]));
        // Five release time constants should recover over 99% of unity.
        assert!((recovery.last().unwrap() - 0.5).abs() < 0.003);
        output.process(&mut [&mut [4.0]], 0.5);
        output.reset();
        let mut quiet = [0.5];
        output.process(&mut [&mut quiet], 0.5);
        assert_eq!(quiet, [0.5]);
    }
}

#[test]
fn overload_and_recovery_are_independent_of_block_size() {
    let source: Vec<_> = (0..4096)
        .map(|i| if i % 503 == 0 { -4.0 } else { 0.25 })
        .collect();
    let mut whole = source.clone();
    Output::new(48000.0).process(&mut [&mut whole], 0.5);
    for size in [1, 17, 256] {
        let mut partitioned = source.clone();
        let mut output = Output::new(48000.0);
        for block in partitioned.chunks_mut(size) {
            output.process(&mut [block], 0.5);
        }
        assert_eq!(partitioned, whole);
    }
}

#[test]
fn empty_block_preserves_gain_history() {
    let mut output = Output::new(48000.0);
    let mut reference = Output::new(48000.0);
    output.process(&mut [&mut [2.0]], 0.5);
    reference.process(&mut [&mut [2.0]], 0.5);
    output.process(&mut [&mut []], 0.5);
    let mut actual = [0.5];
    let mut expected = actual;
    output.process(&mut [&mut actual], 0.5);
    reference.process(&mut [&mut expected], 0.5);
    assert_eq!(actual, expected);
}

#[test]
fn out_sets_mute_unity_and_boost_before_limiting_and_slews_between_them() {
    let mut output = Output::new(48000.0);
    let mut sample = [0.25, 0.75];
    output.process(&mut [&mut sample], 1.0);
    assert_eq!(sample, [0.5, 0.98]);
    let mut fade = [0.25; 600];
    output.process(&mut [&mut fade], 0.0);
    assert!(fade[0] > 0.30);
    assert!(fade.windows(2).all(|s| (s[0] - s[1]).abs() < 0.002));
    assert_eq!(fade[599], 0.0);
    output.reset();
    let mut unity = [0.25];
    output.process(&mut [&mut unity], 0.5);
    assert_eq!(unity, [0.25]);
}
