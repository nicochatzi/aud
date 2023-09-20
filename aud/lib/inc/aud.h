#ifndef AUD_LIB_BINDINGS
#define AUD_LIB_BINDINGS

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
void *aud_audio_stream_create(void);

/**
 * # Safety
 *
 * Not thread-safe, needs to be called from the same
 * thread that calls `create()`
 */
void aud_audio_stream_set_sources(void *ctx,
                                  const struct aud_audio_device_t *sources,
                                  unsigned int num_sources);

/**
 * # Safety
 *
 * This is thread-safe.
 *
 * The caller must supply the name of this source
 */
void aud_audio_stream_push(void *ctx,
                           char *source_name,
                           const float *deinterleave_data,
                           unsigned int num_samples,
                           unsigned int num_channels);

/**
 * Clean up the Audio Stream
 * instance. Ensure the validity
 * of the pointer, it must have
 * been create by a `create`
 */
void aud_audio_stream_destroy(void *ctx);

#endif /* AUD_LIB_BINDINGS */
