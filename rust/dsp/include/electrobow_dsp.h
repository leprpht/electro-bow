#ifndef ELECTROBOW_DSP_H
#define ELECTROBOW_DSP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque Rust-owned object. Create/destroy only off the audio thread. No API
// call may race another call or external band access on the same object.
// Null handles are accepted as errors/no-ops. Other pointers must be live,
// correctly aligned, and valid for the lengths specified below.
typedef struct EbFilterBank EbFilterBank;

// Returns null for invalid settings or band allocation failure. As with the
// C++ runtime, a fatal allocator failure may terminate the process.
EbFilterBank* eb_filter_bank_create(double rate, int32_t capacity, int32_t channels);
void eb_filter_bank_destroy(EbFilterBank* bank);
void eb_filter_bank_reset(EbFilterBank* bank);
// Continuous spectral attack control, 0..500 ms, on the current split buffers.
int32_t eb_filter_bank_attack(EbFilterBank* bank, double milliseconds);

// Status: 1 = success, 0 = invalid argument. No allocation or locks.
// input/output arrays contain the prepared number of channel pointers.
// Each input has `samples` readable floats; each output has room for the last
// successful split's sample count. Outputs must be mutually disjoint. External
// buffers must not overlap bank storage. Host input/output may reuse storage
// across the separate split and recombine calls. Zero samples permit null I/O.
// Failed split clears the valid sample count and preserves filter history.
int32_t eb_filter_bank_split(EbFilterBank* bank, const float* const* input, int32_t samples);
int32_t eb_filter_bank_recombine(const EbFilterBank* bank, float* const* output);

// Returned memory is borrowed, never freed by C++. Allocation lasts until
// destroy; reacquire pointers after split/reset. Access only between calls, and
// only within the prepared capacity. Indices outside configured bands/channels
// return null. A new split establishes the valid sample count.
float* eb_filter_bank_band(EbFilterBank* bank, int32_t band, int32_t channel);
int32_t eb_filter_bank_samples(const EbFilterBank* bank);
// output must point to three writable doubles, outside bank storage.
int32_t eb_filter_bank_crossovers(const EbFilterBank* bank, double* output);

#ifdef __cplusplus
}
#endif
#endif
