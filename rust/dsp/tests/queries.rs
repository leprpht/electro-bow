//! Host-visible queries through the same public entry points used by the plugin.

use electrobow_dsp::*;
use std::ptr;

#[test]
fn crossover_query_reports_prepared_frequencies() {
    for (rate, expected) in [
        (48_000.0, [200.0, 800.0, 3000.0]),
        (4_000.0, [120.0, 480.0, 1800.0]),
    ] {
        let bank = eb_filter_bank_create(rate, 1, 1);
        assert!(!bank.is_null());
        let mut frequencies = [0.0; 3];
        // SAFETY: the exclusive handle is live and the output holds three doubles.
        unsafe {
            assert_eq!(eb_filter_bank_crossovers(bank, frequencies.as_mut_ptr()), 1);
            eb_filter_bank_destroy(bank);
        }
        assert_eq!(frequencies, expected);
    }
}

#[test]
fn input_meter_tracks_the_louder_channel_before_output_trim_and_clears_on_reset() {
    let bank = eb_filter_bank_create(48_000.0, 1, 2);
    assert!(!bank.is_null());
    let panel = PanelControls {
        out: 0.0,
        ..PanelControls::NEUTRAL
    };
    // SAFETY: each call has exclusive handle access and two valid one-frame buffers.
    unsafe {
        assert_eq!(eb_bow_input_level(ptr::null()), 0.0);
        assert_eq!(eb_bow_input_level(bank), 0.0);
        for (left, right) in [(0.125_f32, -0.75_f32), (-0.75, 0.125)] {
            let mut output = [1.0_f32; 2];
            let pointers = output.each_mut().map(|sample| sample as *mut f32);
            assert_eq!(
                eb_bow_process_panel(
                    bank,
                    [&left as *const f32, &right].as_ptr(),
                    pointers.as_ptr(),
                    1,
                    BowControls::NEUTRAL,
                    panel,
                ),
                1
            );
            assert_eq!(output, [0.0; 2]);
            assert!((eb_bow_input_level(bank) - 0.75).abs() < 1e-6);
            eb_filter_bank_reset(bank);
            assert_eq!(eb_bow_input_level(bank), 0.0);
        }
        eb_filter_bank_destroy(bank);
    }
}

#[test]
fn instrument_rejects_invalid_calls_before_advancing_either_channel() {
    assert!(eb_instrument_create(0., 1, 1).is_null());
    let engine = eb_instrument_create(48000., 2, 2);
    assert!(!engine.is_null());
    let settings = InstrumentControls {
        attack_ms: 0.,
        character: 1.,
        noise: 1.,
        saturation: 1.,
        spectral_movement: 1.,
        out: 0.5,
    };
    let input = [0.2_f32, -0.75];
    let inputs = input.each_ref().map(|v| v as *const f32);
    let mut output = [3.0_f32; 2];
    let outputs = output.each_mut().map(|v| v as *mut f32);
    // SAFETY: valid calls process a single frame per channel. Deliberately
    // invalid dimensions and null pointers must be rejected before buffer access.
    unsafe {
        assert_eq!(
            eb_instrument_process(engine, ptr::null(), ptr::null(), 0, settings),
            1
        );
        for invalid in [
            InstrumentControls {
                attack_ms: f64::NAN,
                ..settings
            },
            InstrumentControls {
                character: f64::INFINITY,
                ..settings
            },
            InstrumentControls {
                noise: f64::NAN,
                ..settings
            },
            InstrumentControls {
                saturation: f64::INFINITY,
                ..settings
            },
            InstrumentControls {
                spectral_movement: f64::NAN,
                ..settings
            },
            InstrumentControls {
                out: f64::NEG_INFINITY,
                ..settings
            },
        ] {
            assert_eq!(
                eb_instrument_process(engine, inputs.as_ptr(), outputs.as_ptr(), 1, invalid),
                0
            );
        }
        assert_eq!(
            eb_instrument_process(engine, inputs.as_ptr(), outputs.as_ptr(), 3, settings),
            0
        );
        assert_eq!(
            eb_instrument_process(
                engine,
                [inputs[0], ptr::null()].as_ptr(),
                outputs.as_ptr(),
                1,
                settings
            ),
            0
        );
        assert_eq!(
            eb_instrument_process(
                engine,
                inputs.as_ptr(),
                [outputs[0], ptr::null_mut()].as_ptr(),
                1,
                settings
            ),
            0
        );
        assert_eq!(eb_instrument_input_level(engine), 0.);
        assert_eq!(output, [3.; 2]);
        assert_eq!(
            eb_instrument_process(engine, inputs.as_ptr(), outputs.as_ptr(), 1, settings),
            1
        );
        let first = output;
        assert_eq!(eb_instrument_input_level(engine), 0.75);
        eb_instrument_reset(engine);
        assert_eq!(eb_instrument_input_level(engine), 0.);
        assert_eq!(
            eb_instrument_process(engine, inputs.as_ptr(), outputs.as_ptr(), 1, settings),
            1
        );
        assert_eq!(output, first);
        eb_instrument_destroy(engine);
    }
}
