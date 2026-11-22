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
