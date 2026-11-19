use crate::*;

// Generated fixtures exercise signal properties, not pitch/event recognition.
fn material(i: usize, rate: f64, kind: usize) -> f32 {
    let t = i as f64 / rate;
    let frequencies: &[f64] = match kind {
        0 | 6 => &[110.0],
        1 => &[82.407, 123.471, 164.814, 207.652, 246.942],
        2 => &[110.0, 116.541, 123.471, 130.813, 138.591, 146.832],
        _ => &[146.832, 220.0, 293.665],
    };
    let envelope = match kind {
        3 => (-30.0 * (t % 0.10)).exp(),  // arpeggio spacing
        4 => (-50.0 * (t % 0.06)).exp(),  // alternate picking
        5 => (-60.0 * (t % 0.035)).exp(), // tremolo picking
        6 => 1.0,                         // long sustain
        7 => {
            if t < 0.05 {
                0.05
            } else {
                1.0
            }
        }
        _ => (-4.0 * t).exp(),
    };
    let x = if kind == 3 {
        let f = frequencies[(t / 0.10) as usize % frequencies.len()];
        (std::f64::consts::TAU * f * t).sin()
    } else {
        frequencies
            .iter()
            .map(|f| (std::f64::consts::TAU * f * t).sin())
            .sum::<f64>()
            / frequencies.len() as f64
    };
    (0.8 * x * envelope) as f32
}

#[test]
fn complete_path_materials_rates_channels_and_controls() {
    for rate in [8000.0, 22050.0, 44100.0, 48000.0, 96000.0, 192000.0] {
        for channels in [1, 2] {
            for kind in 0..8 {
                for (attack, character) in [
                    (0.0, 0.0),
                    (250.0, 0.5),
                    (500.0, 1.0),
                    (0.0, 1.0),
                    (500.0, 0.0),
                ] {
                    let bank = eb_filter_bank_create(rate, 257, channels);
                    assert!(!bank.is_null());
                    let mut offset = 0;
                    let mut energy = 0.0;
                    for size in [1, 17, 64, 257].into_iter().cycle().take(100) {
                        let mut left: Vec<f32> = (0..size)
                            .map(|j| material(offset + j, rate, kind))
                            .collect();
                        let mut right: Vec<f32> = left.iter().map(|x| -x).collect();
                        let input = [left.as_ptr(), right.as_ptr()];
                        let output = [left.as_mut_ptr(), right.as_mut_ptr()];
                        // SAFETY: unique live bank, disjoint channels, in-place buffers of size frames.
                        unsafe {
                            assert_eq!(
                                eb_filter_bank_process(
                                    bank,
                                    input.as_ptr(),
                                    output.as_ptr(),
                                    size as i32,
                                    attack,
                                    character
                                ),
                                1
                            );
                        }
                        for (l, r) in left.iter().zip(&right) {
                            assert!(l.is_finite() && l.abs() <= 0.980001);
                            if channels == 2 {
                                assert!(
                                    (l + r).abs() < 1e-7,
                                    "opposite-polarity stereo must not cancel in detector"
                                );
                            }
                            energy += f64::from(*l).powi(2);
                        }
                        offset += size;
                    }
                    assert!(energy > 0.01);
                    // SAFETY: uniquely owned live handle.
                    unsafe {
                        eb_filter_bank_destroy(bank);
                    }
                }
            }
        }
    }
}

#[test]
fn overload_fault_recovery_and_long_sustain() {
    let bank = eb_filter_bank_create(48000.0, 256, 2);
    assert!(!bank.is_null());
    for block in 0..2000 {
        let mut left = [0.0f32; 256];
        let mut right = [0.0f32; 256];
        for (i, x) in left.iter_mut().enumerate() {
            *x = if block == 0 {
                [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX][i % 4]
            } else {
                0.2 * (0.07 * (block * 256 + i) as f32).sin()
            };
            right[i] = *x * 0.5;
        }
        let input = [left.as_ptr(), right.as_ptr()];
        let output = [left.as_mut_ptr(), right.as_mut_ptr()];
        // SAFETY: valid handle and 256-frame disjoint in-place channels.
        unsafe {
            assert_eq!(
                eb_filter_bank_process(bank, input.as_ptr(), output.as_ptr(), 256, 0.0, 0.0),
                1
            );
        }
        for (&l, &r) in left.iter().zip(&right) {
            assert!(l.is_finite() && r.is_finite() && l.abs() <= 0.980001 && r.abs() <= 0.980001);
            // Extreme input clamping intentionally changes channel ratios.
            // Once that fault and IIR decay have passed, linked gain must
            // preserve the valid stereo ratio throughout recovery.
            if block > 10 {
                assert!((l * 0.5 - r).abs() < 1e-7);
            }
        }
        if block == 1999 {
            assert!(
                left.iter().any(|x| x.abs() > 0.19),
                "protection must recover"
            );
        }
    }
    // SAFETY: unique live handle.
    unsafe {
        eb_filter_bank_destroy(bank);
    }
}

#[test]
fn sustained_audio_with_repeated_control_changes() {
    let bank = eb_filter_bank_create(48000.0, 257, 2);
    assert!(!bank.is_null());
    for block in 0..2000 {
        let mut left = [0.0f32; 257];
        let mut right = [0.0f32; 257];
        for i in 0..257 {
            left[i] = 0.7 * ((block * 257 + i) as f32 * 0.03).sin();
            right[i] = left[i];
        }
        let input = [left.as_ptr(), right.as_ptr()];
        let output = [left.as_mut_ptr(), right.as_mut_ptr()];
        let amount = (block % 101) as f64 / 100.0;
        // SAFETY: exclusive handle and disjoint in-place channel arrays.
        unsafe {
            assert_eq!(
                eb_filter_bank_process(
                    bank,
                    input.as_ptr(),
                    output.as_ptr(),
                    257,
                    500.0 * amount,
                    1.0 - amount
                ),
                1
            );
        }
        assert_eq!(left, right);
        assert!(left.iter().all(|x| x.is_finite() && x.abs() <= 0.980001));
    }
    // SAFETY: unique handle, no further access.
    unsafe {
        eb_filter_bank_destroy(bank);
    }
}

#[test]
fn output_guard_links_gain_during_finite_overload() {
    let mut protection = crate::output::Output::new(48000.0);
    let mut left = [2.0, -4.0, 0.2];
    let mut right = [1.0, -2.0, 0.1];
    protection.process(&mut [&mut left, &mut right]);
    for (l, r) in left.into_iter().zip(right) {
        assert!(l.abs() <= 0.980001);
        assert!((l * 0.5 - r).abs() < 1e-7);
    }
}
