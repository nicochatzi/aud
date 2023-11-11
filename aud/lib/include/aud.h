#ifndef AUD_LIB_BINDINGS
#define AUD_LIB_BINDINGS

#define AudioPacketSequence_NUM_BUFFER_PACKETS 4

typedef enum FfiAudioTransmitterResult {
  NoError = 0,
  AudioPushed,
  NoSourceCurrentlySelected,
  OtherSourceSelected,
  FailedToConnectToSocket,
  FailedToParseInputSocket,
  FailedToParseOutputAddress,
  InvalidSourcePointer,
  FailedToParseAudioSource,
  InvalidTransmitterPointer,
} FfiAudioTransmitterResult;

typedef struct FfiAudioTransmitter FfiAudioTransmitter;

typedef struct aud_udp_socket_t {
  const char *port_name;
  unsigned int port_name_length;
} aud_udp_socket_t;

typedef struct aud_audio_device_t {
  const char *name;
  unsigned int num_channels;
} aud_audio_device_t;

typedef struct aud_transmitter_initializer_t {
  struct aud_udp_socket_t input_socket;
  struct aud_udp_socket_t output_socket;
  const struct aud_audio_device_t *sources;
  unsigned int num_sources;
} aud_transmitter_initializer_t;

/**
 * Creates an audio transmitter based on the provided initializer.
 *
 * This function attempts to create a `FfiAudioTransmitter` using the information
 * provided in the `aud_transmitter_initializer_t` struct. It sets up the necessary
 * resources such as input/output sockets and audio sources, and initializes a new
 * audio transmitter which is returned through an out parameter.
 *
 * # Parameters
 * - `initializer`: A struct containing the necessary information to initialize
 *   the audio transmitter.
 * - `transmitter_out`: A pointer to a pointer where the address of the newly
 *   created `FfiAudioTransmitter` will be stored. It's the caller's responsibility
 *   to later free this memory.
 *
 * # Returns
 * - A `FfiAudioTransmitterResult` enum value indicating the result of the operation.
 *   `FfiAudioTransmitterResult::NoError` indicates success, while other values
 *   indicate specific errors that occurred.
 *
 * # Safety
 * This function is `unsafe` as it involves dereferencing raw pointers provided
 * by the caller, and because it returns a heap-allocated object to the caller
 * which must be properly freed to avoid a memory leak.
 *
 * # Example (C code)
 * ```c
 * aud_transmitter_initializer_t initializer = { ... };
 * FfiAudioTransmitter* transmitter = NULL;
 * FfiAudioTransmitterResult retval = aud_audio_transmitter_create(initializer, &transmitter);
 * if (retval == NoError) {
 *     // Use transmitter...
 * } else {
 *     // Handle error...
 * }
 * // Eventually free the transmitter using `aud_audio_transmitter_destroy(&transmitter)`
 * ```
 */
enum FfiAudioTransmitterResult aud_audio_transmitter_create(struct aud_transmitter_initializer_t initializer,
                                                            struct FfiAudioTransmitter **transmitter_out);

/**
 * Pushes an interleaved audio buffer to the specified transmitter.
 *
 * This function takes a buffer of audio data, extracts the specified channels,
 * and sends the resulting buffer to the transmitter for further processing.
 *
 * Note that if the remote has not selected this device the audio
 * will not actually be pushed and this function will be reasonably
 * cheap. Suitable for Debug or RelWithDebInfo-style builds.
 *
 * # Parameters
 * - `transmitter`: A pointer to the `FfiAudioTransmitter` instance.
 * - `source_name`: A pointer to a null-terminated string representing the name of the audio source.
 * - `interleaved_buffer`: A pointer to the buffer containing interleaved audio data.
 * - `num_frames`: The number of frames in the audio buffer.
 * - `num_channels`: The total number of channels in the audio buffer.
 *
 * # Returns
 * - `FfiAudioTransmitterResult::AudioPushed` on success.
 * - `FfiAudioTransmitterResult::InvalidTransmitterPointer` if the `transmitter` pointer is null.
 * - `FfiAudioTransmitterResult::NoSourceCurrentlySelected` if no audio source is selected or the transmitter is not accessible.
 * - `FfiAudioTransmitterResult::InvalidSourcePointer` if the `source_name` pointer could not be converted to a string.
 * - `FfiAudioTransmitterResult::OtherSourceSelected` if a different audio source is currently selected.
 *
 * # Safety
 * This function is unsafe as it operates on raw pointers from FFI, and requires the caller to ensure that the provided pointers are valid,
 * the audio buffer contains the expected number of frames and channels, and the `transmitter` has been properly initialized.
 *
 * # Example (C code)
 * ```c
 * // Assuming transmitter was previously created with aud_audio_transmitter_create...
 * FfiAudioTransmitterResult retval = aud_audio_transmitter_push(transmitter, "My Audio Source", interleaved_buffer, num_frames, num_channels);
 * if (retval != FfiAudioTransmitterResult::AudioPushed) {
 *    // Handle error..
 *    return;
 * }
 * ```
 *
 */
enum FfiAudioTransmitterResult aud_audio_transmitter_push(struct FfiAudioTransmitter *transmitter,
                                                          char *source_name,
                                                          const float *interleaved_buffer,
                                                          unsigned int num_frames,
                                                          unsigned int num_channels);

/**
 * Frees the resources associated with a previously created `FfiAudioTransmitter`.
 *
 * This function is responsible for cleaning up the resources associated with a
 * `FfiAudioTransmitter` instance. It should be called once for every successful
 * call to `aud_audio_transmitter_create`.
 *
 * # Parameters
 * - `transmitter`: A pointer to the `FfiAudioTransmitter` to be destroyed.
 *
 * # Safety
 * This function is `unsafe` because it performs deallocation and because improper
 * use can lead to memory leaks or other undefined behavior (e.g., if called
 * more than once with the same pointer or if the pointer was not previously
 * returned by `aud_audio_transmitter_create`).
 *
 * # Example (C code)
 * ```c
 * // Assuming transmitter was previously created with aud_audio_transmitter_create...
 * aud_audio_transmitter_destroy(transmitter);
 * ```
 */
void aud_audio_transmitter_destroy(struct FfiAudioTransmitter *transmitter);

#endif /* AUD_LIB_BINDINGS */
