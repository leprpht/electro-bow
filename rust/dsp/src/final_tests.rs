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

fn render_complete(
    input: &[f32],
    rate: f64,
    chunk: usize,
    attack: f64,
    character: f64,
) -> Vec<f32> {
    render_controls(
        input,
        rate,
        chunk,
        BowControls {
            attack_ms: attack,
            character,
            ..BowControls::NEUTRAL
        },
    )
}

fn render_controls(input: &[f32], rate: f64, chunk: usize, controls: BowControls) -> Vec<f32> {
    let bank = eb_filter_bank_create(rate, chunk as i32, 1);
    assert!(!bank.is_null());
    let mut output = input.to_vec();
    for block in output.chunks_mut(chunk) {
        let input = [block.as_ptr()];
        let output = [block.as_mut_ptr()];
        // SAFETY: exclusive handle and one valid in-place channel of block.len() frames.
        unsafe {
            assert_eq!(
                eb_bow_process(
                    bank,
                    input.as_ptr(),
                    output.as_ptr(),
                    block.len() as i32,
                    controls,
                ),
                1
            );
        }
    }
    // SAFETY: exclusive live handle, no outstanding band borrows.
    unsafe { eb_filter_bank_destroy(bank) };
    output
}

#[test]
fn complete_path_preserves_dynamics_and_block_partitioning() {
    // Isolated-stage scale tests cannot catch interactions with the output
    // protector or Attack -> Character routing. Exercise the public full path.
    let mut worst_scaling_error = 0.0_f32;
    let mut worst_partition_error = 0.0_f32;
    for rate in [8000.0, 44100.0, 48000.0, 96000.0, 192000.0] {
        for kind in 0..8 {
            let input: Vec<_> = (0..(rate * 0.4) as usize)
                .map(|i| material(i, rate, kind) * 0.5)
                .collect();
            let quiet: Vec<_> = input.iter().map(|x| x * 0.1).collect();
            for (attack, character) in [(0.0, 0.0), (250.0, 0.5), (500.0, 1.0)] {
                let loud = render_complete(&input, rate, 257, attack, character);
                let soft = render_complete(&quiet, rate, 257, attack, character);
                let partitioned = render_complete(&input, rate, 17, attack, character);
                for (((&x, &y), &z), &original) in
                    loud.iter().zip(&soft).zip(&partitioned).zip(&input)
                {
                    assert!(x.is_finite() && x.abs() < 0.9, "fixture needs headroom");
                    worst_scaling_error = worst_scaling_error.max((0.1 * x - y).abs());
                    worst_partition_error = worst_partition_error.max((x - z).abs());
                    if attack == 0.0 && character == 0.0 {
                        assert!((x - original).abs() < 2e-7, "neutral reconstruction");
                    }
                }
            }
        }
    }
    assert!(
        worst_scaling_error < 1e-7,
        "playing dynamics below limiting"
    );
    assert!(
        worst_partition_error < 1e-7,
        "host buffer size independence"
    );
    println!(
        "Full-path scaling error={worst_scaling_error:.9e}, partition error={worst_partition_error:.9e}"
    );
}

#[test]
fn sustain_lifts_falling_notes_and_returns_to_unity_at_the_noise_floor() {
    let mut source: Vec<_> = (0..96000)
        .map(|i| {
            let t = i as f64 / 48000.0;
            let amplitude = if t < 1.0 {
                0.4 * (-4.0 * t).exp()
            } else {
                0.0005
            };
            (amplitude * (t * std::f64::consts::TAU * 110.0).sin()) as f32
        })
        .collect();
    let wet = render_controls(
        &source,
        48000.0,
        97,
        BowControls {
            sustain: 1.0,
            ..BowControls::NEUTRAL
        },
    );
    let energy = |x: &[f32]| x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>();
    let ratio = |r: std::ops::Range<usize>| energy(&wet[r.clone()]) / energy(&source[r]);
    assert!(ratio(0..1000) < 1.15);
    assert!(ratio(24000..36000) > 3.0);
    assert!((ratio(90000..96000) - 1.0).abs() < 0.01);
    source.fill(0.0);
    let controls = BowControls {
        sustain: 1.0,
        pressure: 1.0,
        speed: 1.0,
        friction: 1.0,
        character: 1.0,
        resonance_ms: 1000.0,
        attack_ms: 500.0,
    };
    assert!(
        render_controls(&source, 48000.0, 61, controls)
            .iter()
            .all(|x| *x == 0.0)
    );
}

#[test]
fn resonance_decay_tracks_time_and_all_controls_are_partition_independent() {
    for rate in [8000.0, 48000.0, 192000.0] {
        let mut signal = vec![0.0; (rate * 1.7) as usize];
        let stop = (rate * 0.23) as usize;
        for (i, x) in signal[..stop].iter_mut().enumerate() {
            *x = material(i, rate, 1);
        }
        let config = |ms| BowControls {
            resonance_ms: ms,
            sustain: 0.8,
            pressure: 0.6,
            speed: 0.25,
            friction: 0.9,
            character: 0.75,
            attack_ms: 70.0,
        };
        let short = render_controls(&signal, rate, 127, config(120.0));
        let long = render_controls(&signal, rate, 127, config(900.0));
        let split = render_controls(&signal, rate, 17, config(900.0));
        assert_eq!(long, split);
        let peak = |buf: &[f32]| buf.iter().map(|x| x.abs()).fold(0.0_f32, f32::max);
        let tail = (stop + (rate * 0.2) as usize)..(stop + (rate * 0.3) as usize);
        assert!(peak(&long[tail.clone()]) > 8.0 * peak(&short[tail]));
        assert!(peak(&long[(rate * 1.6) as usize..]) < 2e-5);
    }
}

#[test]
fn contact_controls_change_compression_tilt_and_third_harmonic() {
    let sine = |frequency: f64, level: f64| {
        (0..24000)
            .map(|i| {
                (level * (std::f64::consts::TAU * frequency * i as f64 / 48000.0).sin()) as f32
            })
            .collect::<Vec<_>>()
    };
    let amplitude = |input: &[f32], hz: f64, config| {
        let out = render_controls(input, 48000.0, 59, config);
        let (mut re, mut im) = (0.0, 0.0);
        for (i, x) in out[12000..].iter().enumerate() {
            let phase = std::f64::consts::TAU * hz * i as f64 / 48000.0;
            re += f64::from(*x) * phase.cos();
            im += f64::from(*x) * phase.sin();
        }
        2.0 * re.hypot(im) / 12000.0
    };
    let config = BowControls::NEUTRAL;
    let pressed = BowControls {
        pressure: 1.0,
        ..config
    };
    let soft = amplitude(&sine(100.0, 0.05), 100.0, pressed);
    let hard = amplitude(&sine(100.0, 0.5), 100.0, pressed);
    assert!(hard > soft * 4.0 && hard < soft * 7.0);
    let bright = sine(5000.0, 0.1);
    let at_speed = |speed| amplitude(&bright, 5000.0, BowControls { speed, ..config });
    assert!(at_speed(1.0) > 2.0 * at_speed(0.0));
    let low = sine(100.0, 0.2);
    assert!(amplitude(&low, 300.0, config) < 1e-6);
    let harmonic = amplitude(
        &low,
        300.0,
        BowControls {
            friction: 1.0,
            ..config
        },
    );
    assert!((0.01..0.06).contains(&harmonic));
}

#[test]
fn long_body_decay_keeps_sustained_harmonics_present() {
    // The strongest cancellation in the untrimmed body return was near
    // 807 Hz. Exercise that mode and adjacent guitar harmonics at steady state.
    for frequency in [287.0, 540.0, 806.75, 1008.0] {
        let signal: Vec<_> = (0..72000)
            .map(|i| (0.1 * (std::f64::consts::TAU * frequency * i as f64 / 48000.0).sin()) as f32)
            .collect();
        let wet = render_controls(
            &signal,
            48000.0,
            113,
            BowControls {
                resonance_ms: 1000.0,
                ..BowControls::NEUTRAL
            },
        );
        let energy = |buf: &[f32]| {
            buf[48000..]
                .iter()
                .map(|&x| f64::from(x).powi(2))
                .sum::<f64>()
        };
        let ratio = energy(&wet) / energy(&signal);
        assert!(
            (0.25..1.6).contains(&ratio),
            "{frequency} Hz power ratio {ratio}"
        );
    }
}

#[test]
fn restored_controls_sound_the_same_with_or_without_silent_preroll() {
    let config = BowControls {
        attack_ms: 35.0,
        pressure: 0.9,
        speed: 0.05,
        friction: 0.8,
        character: 0.9,
        sustain: 0.7,
        resonance_ms: 500.0,
    };
    let input: Vec<_> = (0..12000).map(|i| material(i, 48000.0, 1)).collect();
    let mut padded = vec![0.0; 48000];
    padded.extend_from_slice(&input);
    let from_rest = render_controls(&input, 48000.0, 97, config);
    let from_preroll = render_controls(&padded, 48000.0, 97, config);
    let error = from_rest
        .iter()
        .zip(&from_preroll[48000..])
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(error < 1e-6, "first stroke depends on preroll: {error}");
}
