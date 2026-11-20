
#ifndef ELECTROBOW_DSP_H
#define ELECTROBOW_DSP_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @file electrobow_dsp.h
 * @brief C interface to the Rust ElectroBow DSP filter bank.
 *
 * The filter bank provides two processing workflows:
 *
 * 1. Full processing:
 *      create -> process -> destroy
 *
 * 2. External band processing:
 *      create -> split -> band -> modify -> recombine -> destroy
 *
 * The filter bank owns its internal DSP state and band buffers.
 * Callers must not allocate, copy, or free the underlying object.
 *
 * THREAD SAFETY
 * -------------
 * Create and destroy must run outside the real-time audio thread.
 *
 * Calls operating on the same bank must not overlap. This includes
 * external reads or writes to borrowed band buffers.
 *
 * Successful processing operations are intended for real-time use
 * and do not allocate memory or acquire locks.
 *
 * POINTER VALIDITY
 * ----------------
 * Null bank handles are handled as errors or no-ops.
 *
 * All other non-null pointers must refer to live, correctly aligned
 * memory with sufficient readable or writable storage.
 *
 * The caller is responsible for respecting buffer lengths, channel
 * counts, and the documented aliasing restrictions.
 */

/**
 * @brief Opaque Rust-owned filter-bank instance.
 *
 * Instances must be created using eb_filter_bank_create()
 * and released using eb_filter_bank_destroy().
 *
 * Never dereference this type from C or C++.
 */
typedef struct EbFilterBank EbFilterBank;

/**
 * @brief Create and initialize a filter bank.
 *
 * Allocates internal DSP state and per-band sample buffers.
 *
 * @param rate      Sample rate in Hz.
 * @param capacity  Maximum number of frames per processing block.
 * @param channels  Number of audio channels.
 *
 * @return A new filter-bank handle, or NULL if configuration
 *         validation or band allocation fails.
 *
 * @note Call outside the real-time audio thread.
 * @note A fatal allocator failure may terminate the process,
 *       consistent with the C++ runtime's allocation behavior.
 */
EbFilterBank* eb_filter_bank_create(double rate, int32_t capacity, int32_t channels);

/**
 * @brief Destroy a filter bank and release its resources.
 *
 * Invalidates all band pointers previously obtained from the bank.
 *
 * @param bank Filter-bank handle, or NULL for a no-op.
 *
 * @warning Must not overlap processing or borrowed-buffer access.
 * @note Call outside the real-time audio thread.
 */
void eb_filter_bank_destroy(EbFilterBank* bank);

/**
 * @brief Reset the filter bank's processing state.
 *
 * Use when restarting a stream or intentionally discarding
 * accumulated filter history.
 *
 * Previously borrowed band pointers should be reacquired
 * before further external access.
 *
 * @param bank Filter-bank handle, or NULL for a no-op.
 *
 * @note Reset does not destroy the bank or release its capacity.
 */
void eb_filter_bank_reset(EbFilterBank* bank);

/**
 * @brief Process audio through the complete DSP path.
 *
 * Applies the filter-bank processing path using the supplied
 * attack and character controls.
 *
 * The output includes a stereo-linked sample-peak ceiling
 * with a maximum target amplitude of 0.98.
 *
 * @param bank       Initialized filter-bank handle.
 * @param input      Array of input channel pointers.
 * @param output     Array of output channel pointers.
 * @param samples    Number of frames to process.
 * @param attack     Finite attack-control value.
 * @param character  Finite character-control value.
 *
 * Each input channel must contain at least 'samples' readable
 * float values. Each output channel must provide sufficient
 * writable storage.
 *
 * Input and output buffers may alias, allowing in-place
 * processing when the other buffer rules are satisfied.
 *
 * @return 1 on success, 0 for invalid arguments.
 *
 * @note Does not allocate memory or acquire locks.
 * @warning Do not call concurrently with another operation
 *          on the same bank.
 */
int32_t eb_filter_bank_process(EbFilterBank* bank, const float* const* input, float* const* output,
                               int32_t samples, double attack, double character);

/** Full effect settings in physical units. Speed is neutral at 0.5. */
typedef struct EbBowControls {
    double attack_ms;    /**< 0..500 ms */
    double character;    /**< 0..1 */
    double sustain;      /**< 0..1 */
    double resonance_ms; /**< 0..1000 ms */
    double pressure;     /**< 0..1 */
    double speed;        /**< 0..1 */
    double friction;     /**< 0..1 */
} EbBowControls;

/** Process the seven-control effect. Same buffers, aliasing and return
 * contract as eb_filter_bank_process. Non-finite controls are rejected. */
int32_t eb_bow_process(EbFilterBank* bank, const float* const* input, float* const* output,
                       int32_t samples, EbBowControls controls);

/**
 * @brief Apply the coordinated spectral/friction character control.
 *
 * Operates on the bank's current split-band buffers.
 *
 * @param bank    Initialized filter-bank handle.
 * @param amount  Normalized character value in [0.0, 1.0].
 *
 * @return 1 on success, 0 for invalid arguments.
 *
 * @note The exact spectral and friction response is determined
 *       by the DSP implementation.
 */
int32_t eb_filter_bank_character(EbFilterBank* bank, double amount);

/**
 * @brief Apply continuous spectral attack shaping.
 *
 * Operates on the bank's current split-band buffers.
 *
 * @param bank          Initialized filter-bank handle.
 * @param milliseconds  Attack time in [0.0, 500.0] ms.
 *
 * @return 1 on success, 0 for invalid arguments.
 */
int32_t eb_filter_bank_attack(EbFilterBank* bank, double milliseconds);

/**
 * @brief Split an audio block into internal frequency-band buffers.
 *
 * A successful split establishes the valid sample count for
 * subsequent band access and recombination.
 *
 * The split buffers remain owned by the filter bank.
 *
 * @param bank     Initialized filter-bank handle.
 * @param input    Array containing one pointer per channel.
 * @param samples  Number of frames, within prepared capacity.
 *
 * @return 1 on success, 0 for invalid arguments.
 *
 * @note No allocation or locking occurs.
 * @note A zero-length block permits null input pointers.
 *
 * @warning External input buffers must not overlap internal
 *          filter-bank storage.
 *
 * @par Failure behavior
 * An unsuccessful split clears the valid sample count but
 * preserves accumulated filter history.
 */
int32_t eb_filter_bank_split(EbFilterBank* bank, const float* const* input, int32_t samples);

/**
 * @brief Reconstruct audio from the current split-band buffers.
 *
 * Recombines the current band contents, including modifications
 * made by the caller after a successful split.
 *
 * @param bank    Initialized filter-bank handle.
 * @param output  Array containing one writable pointer per channel.
 *
 * Each output channel must have room for the sample count
 * established by the last successful split.
 *
 * Output channel buffers must be mutually disjoint and must
 * not overlap internal bank storage.
 *
 * Host-owned input and output storage may be reused across
 * separate split and recombine calls.
 *
 * @return 1 on success, 0 for invalid arguments.
 *
 * @note No allocation or locking occurs.
 * @note Zero valid samples permit null output pointers.
 */
int32_t eb_filter_bank_recombine(const EbFilterBank* bank, float* const* output);

/**
 * @brief Obtain writable access to an internal frequency band.
 *
 * The returned pointer refers directly to Rust-owned sample
 * storage. Modifications affect subsequent operations using
 * the current split-band data.
 *
 * @param bank     Initialized filter-bank handle.
 * @param band     Zero-based frequency-band index.
 * @param channel  Zero-based channel index.
 *
 * @return Pointer to the band's sample buffer, or NULL if
 *         the handle or indices are invalid.
 *
 * @par Lifetime and access
 * - Never free the returned pointer.
 * - Storage allocation remains owned by the bank until destroy.
 * - Access only between filter-bank API calls.
 * - Reacquire the pointer after split or reset.
 * - Never access beyond the configured capacity.
 *
 * The valid audio sample count is obtained separately using
 * eb_filter_bank_samples().
 *
 * @warning Concurrent external access and API calls on the
 *          same bank are prohibited.
 */
float* eb_filter_bank_band(EbFilterBank* bank, int32_t band, int32_t channel);

/**
 * @brief Query the number of currently valid split samples.
 *
 * The count is established by a successful split.
 * A failed split clears the count.
 *
 * @param bank Initialized filter-bank handle.
 *
 * @return Current valid sample count.
 */
int32_t eb_filter_bank_samples(const EbFilterBank* bank);

/**
 * @brief Retrieve the filter bank's three crossover frequencies.
 *
 * @param bank    Initialized filter-bank handle.
 * @param output  Pointer to storage for at least three doubles.
 *
 * The output storage must not overlap internal bank memory.
 *
 * @return 1 on success, 0 for invalid arguments.
 *
 * @note No allocation or locking occurs.
 */
int32_t eb_filter_bank_crossovers(const EbFilterBank* bank, double* output);

#ifdef __cplusplus
}
#endif

#endif /* ELECTROBOW_DSP_H */
