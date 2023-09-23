#ifndef AUD_LIB_BINDINGS
#define AUD_LIB_BINDINGS

#define AudioPacketSequence_NUM_BUFFER_PACKETS 4

typedef struct aud_audio_device_t {
  const char *name;
  unsigned int num_channels;
} aud_audio_device_t;

/**
 * Create an Audio Stream instance.
 *
 * The library consumer should
 * create, setup the sources,
 * then pass a pointer to this
 * stream to any `aud` audio
 * consumer
 */
void *aud_audio_provider_create(void);

/**
 * # Safety
 *
 * Not thread-safe, needs to be called from the same
 * thread that calls `create()`
 */
void aud_audio_provider_set_sources(void *ctx,
                                    const struct aud_audio_device_t *sources,
                                    unsigned int num_sources);

/**
 * # Safety
 *
 * This is thread-safe.
 *
 * The caller must supply the name of this source
 */
void aud_audio_provider_push(void *ctx,
                             char *source_name,
                             const float *interleaved_buffer,
                             unsigned int num_frames,
                             unsigned int num_channels);

/**
 * Destroy the instance for clean up.
 *
 * Ensure the validity of the pointer, it must have been create by a `create`
 */
void aud_audio_provider_destroy(void *ctx);

#endif /* AUD_LIB_BINDINGS */
