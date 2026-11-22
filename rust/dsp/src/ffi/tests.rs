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
