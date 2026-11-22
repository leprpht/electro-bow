use super::*;

const NEUTRAL: InstrumentControls = InstrumentControls {
    attack_ms: 0.,
    character: 0.,
    noise: 0.,
    saturation: 0.,
    spectral_movement: 0.,
    out: 0.5,
};

fn chord(i: usize, rate: f64) -> f32 {
    let t = i as f64 / rate;
    [82.4, 110., 146.8, 196., 246.9, 329.6]
        .into_iter()
        .enumerate()
        .map(|(string, hz)| {
            let age = t - 0.003 * string as f64;
            if age < 0. {
                return 0.;
            }
            let phase = std::f64::consts::TAU * hz * age;
            (-2. * age).exp() * (0.08 * phase.sin() + 0.04 * (4. * phase).sin())
        })
        .sum::<f64>() as f32
}

fn mono(
    engine: &mut Instrument,
    input: &[f32],
    block: usize,
    settings: InstrumentControls,
) -> Vec<f32> {
    let mut output = input.to_vec();
    for frames in output.chunks_mut(block) {
        engine.ingest(0, frames);
        engine.render(&mut [frames], settings);
    }
    output
}

fn power(signal: &[f32]) -> f64 {
    signal.iter().map(|&x| f64::from(x).powi(2)).sum::<f64>()
}

#[test]
fn every_dial_reaches_the_chord_and_details_follow_character() {
    let rate = 48_000.;
    let input: Vec<_> = (0..48_000).map(|i| chord(i, rate)).collect();
    let mut engine = Instrument::new(rate, 257, 1).unwrap();
    let mut render = |s| {
        engine.reset();
        mono(&mut engine, &input, 257, s)
    };
    let active = InstrumentControls {
        character: 1.,
        ..NEUTRAL
    };
    let base = render(active);
    for settings in [
        NEUTRAL,
        InstrumentControls {
            attack_ms: 250.,
            ..active
        },
        InstrumentControls {
            noise: 1.,
            ..active
        },
        InstrumentControls {
            saturation: 1.,
            ..active
        },
        InstrumentControls {
            spectral_movement: 1.,
            ..active
        },
        InstrumentControls {
            out: 0.25,
            ..active
        },
    ] {
        let changed = render(settings);
        let diff: Vec<_> = changed.iter().zip(&base).map(|(a, b)| a - b).collect();
        assert!(power(&diff) > power(&base) * 1e-5);
    }
    let transparent = render(InstrumentControls {
        noise: 1.,
        saturation: 1.,
        spectral_movement: 1.,
        ..NEUTRAL
    });
    assert!(
        transparent
            .iter()
            .zip(input)
            .all(|(a, b)| (a - b).abs() < 1e-7)
    );
}

#[test]
fn all_control_corners_are_bounded_and_replay_across_block_sizes() {
    for rate in [8000., 44100., 192000.] {
        let input: Vec<_> = (0..4096).map(|i| 6.0 * chord(i, rate)).collect();
        let mut engine = Instrument::new(rate, 4096, 1).unwrap();
        for mask in 0..64 {
            let bit = |i: u32| f64::from((mask >> i) & 1);
            let controls = InstrumentControls {
                attack_ms: 500. * bit(0),
                character: bit(1),
                noise: bit(2),
                saturation: bit(3),
                spectral_movement: bit(4),
                out: bit(5),
            };
            engine.reset();
            let whole = mono(&mut engine, &input, 4096, controls);
            assert!(whole.iter().all(|x| x.is_finite() && x.abs() <= 0.980001));
            if controls.out == 0. {
                assert!(whole.iter().all(|&x| x == 0.));
            }
            engine.reset();
            assert_eq!(whole, mono(&mut engine, &input, 17, controls));
        }
    }
}

#[test]
fn control_steps_are_continuous_on_an_already_ringing_body() {
    let rate = 48_000.;
    let mut reference = Instrument::new(rate, 256, 1).unwrap();
    let mut automated = Instrument::new(rate, 256, 1).unwrap();
    let extremes = InstrumentControls {
        attack_ms: 500.,
        character: 1.,
        noise: 1.,
        saturation: 1.,
        spectral_movement: 1.,
        out: 1.,
    };
    let mut previous = NEUTRAL;
    for block in 0..120 {
        let input: Vec<_> = (0..256)
            .map(|i| chord((i + 256 * block) % 8192, rate))
            .collect();
        let next = if block % 2 == 0 { NEUTRAL } else { extremes };
        // Delay every automation step by one sample on the reference path.
        // A discontinuous change produces a spike between the two renders.
        let old = mono(&mut reference, &input[..1], 1, previous);
        let rest = mono(&mut reference, &input[1..], 255, next);
        let changed = mono(&mut automated, &input, 256, next);
        assert!(
            old.iter()
                .chain(&rest)
                .zip(changed)
                .all(|(a, b)| (a - b).abs() < 0.004)
        );
        previous = next;
    }
}

#[test]
fn full_body_is_causal_channel_isolated_and_decays_within_reported_tail() {
    for rate in [4000., 48000., 192000.] {
        let frames = (rate * 0.30) as usize;
        let mut engine = Instrument::new(rate, frames, 2).unwrap();
        let mut left = vec![0.; frames];
        let mut right = vec![0.; frames];
        left[0] = 0.5;
        engine.ingest(0, &left);
        engine.ingest(1, &right);
        engine.render(
            &mut [&mut left, &mut right],
            InstrumentControls {
                character: 1.,
                noise: 1.,
                saturation: 1.,
                spectral_movement: 1.,
                ..NEUTRAL
            },
        );
        assert!(left[0] > 0.05);
        assert!(power(&left[1..]) > 1e-5);
        assert!(left[frames * 9 / 10..].iter().all(|x| x.abs() < 1e-8));
        assert!(right.iter().all(|&x| x == 0.));
    }
}

#[test]
fn overlapping_notes_add_linearly_with_the_saturator_and_swell_at_zero() {
    let mut engine = Instrument::new(48000., 127, 1).unwrap();
    let a: Vec<_> = (0..16384)
        .map(|i| 0.12 * (i as f32 * 0.014).sin())
        .collect();
    let b: Vec<_> = (0..a.len())
        .map(|i| 0.10 * (i as f32 * 0.021).sin())
        .collect();
    let chord: Vec<_> = a.iter().zip(&b).map(|(x, y)| x + y).collect();
    let mut play = |input: &[f32]| {
        engine.reset();
        mono(
            &mut engine,
            input,
            127,
            InstrumentControls {
                character: 1.,
                noise: 1.,
                spectral_movement: 1.,
                ..NEUTRAL
            },
        )
    };
    for ((x, y), z) in play(&a).iter().zip(play(&b)).zip(play(&chord)) {
        assert!((x + y - z).abs() < 2e-7);
    }
}
