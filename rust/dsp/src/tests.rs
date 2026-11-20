use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

// Thread-local counting keeps parallel tests from contaminating the real-time
// assertion. Count allocations, reallocations AND deallocations during DSP.
struct CountingAllocator;
thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static HEAP_OPERATIONS: Cell<usize> = const { Cell::new(0) };
}
fn count_heap_operation() {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = HEAP_OPERATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
// SAFETY: every operation is forwarded unchanged to the system allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_heap_operation();
        // SAFETY: GlobalAlloc caller provides a valid layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        count_heap_operation();
        // SAFETY: GlobalAlloc caller guarantees the allocation/layout pair.
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_heap_operation();
        // SAFETY: forwarded allocation/layout and requested size are valid.
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn rejects_invalid_preparation() {
    for rate in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(eb_filter_bank_create(rate, 64, 2).is_null());
    }
    for (capacity, channels) in [(0, 2), (-1, 2), (64, 0), (64, -1), (64, 3)] {
        assert!(eb_filter_bank_create(48000.0, capacity, channels).is_null());
    }
}

#[test]
fn ffi_rejects_bad_arguments_without_changing_history() {
    // SAFETY: all non-null pointers below refer to live arrays/handles with the
    // advertised lengths. Invalid arguments exercise explicit checked errors.
    unsafe {
        eb_filter_bank_destroy(ptr::null_mut());
        eb_filter_bank_reset(ptr::null_mut());
        assert_eq!(eb_filter_bank_samples(ptr::null()), 0);
        assert_eq!(eb_filter_bank_split(ptr::null_mut(), ptr::null(), 0), 0);
        assert_eq!(eb_filter_bank_recombine(ptr::null(), ptr::null()), 0);
        assert!(eb_filter_bank_band(ptr::null_mut(), 0, 0).is_null());
        let bank = eb_filter_bank_create(48000.0, 64, 2);
        assert!(!bank.is_null());
        let signal = [1.0; 64];
        let input = [signal.as_ptr(), signal.as_ptr()];
        assert_eq!(eb_filter_bank_split(bank, input.as_ptr(), 64), 1);
        let history = (*bank).states;
        assert_eq!(eb_filter_bank_split(bank, input.as_ptr(), 65), 0);
        assert_eq!(eb_filter_bank_samples(bank), 0);
        assert_eq!(eb_filter_bank_split(bank, input.as_ptr(), -1), 0);
        assert_eq!(eb_filter_bank_split(bank, ptr::null(), 1), 0);
        let invalid_input = [signal.as_ptr(), ptr::null()];
        assert_eq!(eb_filter_bank_split(bank, invalid_input.as_ptr(), 64), 0);
        assert_eq!((*bank).states, history);
        assert_eq!(eb_filter_bank_split(bank, ptr::null(), 0), 1);
        assert_eq!(eb_filter_bank_recombine(bank, ptr::null()), 1);
        assert!(eb_filter_bank_band(bank, -1, 0).is_null());
        assert!(eb_filter_bank_band(bank, 4, 0).is_null());
        assert!(eb_filter_bank_band(bank, 0, 2).is_null());
        assert_eq!(eb_filter_bank_crossovers(bank, ptr::null_mut()), 0);
        assert_eq!(eb_filter_bank_split(bank, input.as_ptr(), 64), 1);
        assert_eq!(eb_filter_bank_recombine(bank, ptr::null()), 0);
        let mut output = [0.0; 64];
        let invalid_output = [output.as_mut_ptr(), ptr::null_mut()];
        assert_eq!(eb_filter_bank_recombine(bank, invalid_output.as_ptr()), 0);
        assert_eq!(output, [0.0; 64]);
        eb_filter_bank_destroy(bank);
    }
}

#[test]
fn ffi_round_trip_and_channel_isolation() {
    for rate in [
        4000.0, 8000.0, 44100.0, 48000.0, 96000.0, 192000.0, 384000.0,
    ] {
        for channels in [1, 2] {
            // SAFETY: handles are exclusively owned, arrays have exactly the
            // requested capacity, band access occurs only between API calls.
            unsafe {
                let bank = eb_filter_bank_create(rate, 256, channels);
                assert!(!bank.is_null());
                let mut signal = [0.0; 256];
                for (index, sample) in signal.iter_mut().enumerate() {
                    *sample = ((index * 173 % 257) as f32 - 128.0) / 128.0;
                }
                let silence = [0.0; 256];
                let input = [signal.as_ptr(), silence.as_ptr()];
                let mut left = [0.0; 256];
                let mut right = [0.0; 256];
                let output = [left.as_mut_ptr(), right.as_mut_ptr()];
                for samples in [1, 17, 64, 256] {
                    assert_eq!(eb_filter_bank_split(bank, input.as_ptr(), samples), 1);
                    assert_eq!(eb_filter_bank_recombine(bank, output.as_ptr()), 1);
                    for i in 0..samples as usize {
                        assert!((left[i] - signal[i]).abs() < 2.0e-7);
                    }
                    if channels == 2 {
                        for band in 0..4 {
                            let data = eb_filter_bank_band(bank, band, 1);
                            assert!(
                                slice::from_raw_parts(data, samples as usize)
                                    .iter()
                                    .all(|&x| x == 0.0)
                            );
                        }
                    }
                }
                eb_filter_bank_reset(bank);
                assert_eq!(eb_filter_bank_samples(bank), 0);
                for band in 0..4 {
                    for channel in 0..channels {
                        assert!(
                            slice::from_raw_parts(eb_filter_bank_band(bank, band, channel), 256)
                                .iter()
                                .all(|&x| x == 0.0)
                        );
                    }
                }
                eb_filter_bank_destroy(bank);
            }
        }
    }
}

#[test]
fn processing_has_no_heap_operations_and_band_addresses_stay_stable() {
    // SAFETY: unique handle, full-capacity stack buffers and pointers used only
    // between calls. Destruction occurs outside the tracked real-time region.
    unsafe {
        let bank = eb_filter_bank_create(48000.0, 256, 2);
        assert!(!bank.is_null());
        let signal = [0.2; 256];
        let input = [signal.as_ptr(), signal.as_ptr()];
        let mut left = [0.0; 256];
        let mut right = [0.0; 256];
        let output = [left.as_mut_ptr(), right.as_mut_ptr()];
        let address = eb_filter_bank_band(bank, 2, 1);
        HEAP_OPERATIONS.with(|count| count.set(0));
        TRACK.with(|track| track.set(true));
        let mut success = true;
        for _ in 0..1000 {
            success &=
                eb_filter_bank_process(bank, input.as_ptr(), output.as_ptr(), 256, 500.0, 1.0) == 1;
            success &= eb_filter_bank_attack(bank, 500.0) == 1;
            success &= eb_filter_bank_character(bank, 1.0) == 1;
            success &= eb_filter_bank_recombine(bank, output.as_ptr()) == 1;
            success &= eb_filter_bank_band(bank, 2, 1) == address;
        }
        eb_filter_bank_reset(bank);
        TRACK.with(|track| track.set(false));
        assert!(success);
        assert_eq!(HEAP_OPERATIONS.with(Cell::get), 0);
        eb_filter_bank_destroy(bank);
    }
}

fn shaped(signal: &[f32], amount: f64, scale: f32, chunk: usize) -> Vec<f32> {
    let mut bank = FilterBank::new(48000.0, chunk, 1).unwrap();
    let mut result = Vec::new();
    for input in signal.chunks(chunk) {
        let input: Vec<_> = input.iter().map(|x| x * scale).collect();
        bank.split_channel(0, &input);
        bank.attack.process(&mut bank.bands, 1, input.len(), amount);
        let mut output = vec![0.0; input.len()];
        bank.recombine_channel(0, &mut output);
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

fn character_render(input: &[f32], amount: f64, scale: f32, chunk: usize, rate: f64) -> Vec<f32> {
    let mut bank = FilterBank::new(rate, chunk, 1).unwrap();
    let mut result = Vec::new();
    for source in input.chunks(chunk) {
        let source: Vec<_> = source.iter().map(|x| x * scale).collect();
        bank.split_channel(0, &source);
        bank.character
            .process(&mut bank.bands, 1, source.len(), amount);
        let mut output = vec![0.0; source.len()];
        bank.recombine_channel(0, &mut output);
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
            bank.character.process(&mut bank.bands, 2, 256, amount);
            assert!(bank.bands.iter().flatten().flatten().all(|x| *x == 0.0));
        }
    }
    let signal: Vec<f32> = (0..256).map(|i| (i as f32 * 0.17).sin() * 0.5).collect();
    bank.split_channel(0, &signal);
    bank.split_channel(1, &signal);
    bank.character.process(&mut bank.bands, 2, 256, 1.0);
    for band in &bank.bands {
        assert_eq!(band[0], band[1]);
    }
    let first = bank.bands.clone();
    bank.reset();
    bank.split_channel(0, &signal);
    bank.split_channel(1, &signal);
    bank.character.process(&mut bank.bands, 2, 256, 1.0);
    // Reset restores the envelope and control smoother.
    let mut fresh = FilterBank::new(48000.0, 256, 2).unwrap();
    fresh.split_channel(0, &signal);
    fresh.split_channel(1, &signal);
    fresh.character.process(&mut fresh.bands, 2, 256, 1.0);
    assert_eq!(bank.bands, fresh.bands);
    assert_ne!(first, bank.bands);
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
