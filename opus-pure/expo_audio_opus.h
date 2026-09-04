#ifndef EXPO_AUDIO_OPUS_H
#define EXPO_AUDIO_OPUS_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct RecorderStream RecorderStream;
typedef struct PlayerDecoder PlayerDecoder;

// Recorder API
RecorderStream* opus_recorder_create(
    const char* file_path,
    int32_t sample_rate,
    int32_t channels,
    int32_t bitrate,
    int32_t application
);

int32_t opus_recorder_write_pcm(
    RecorderStream* stream,
    const int16_t* pcm,
    size_t count
);

int32_t opus_recorder_finish(
    RecorderStream* stream,
    uint64_t* out_duration_ms,
    uint64_t* out_file_size
);

void opus_recorder_destroy(RecorderStream* stream);

// Player API
PlayerDecoder* opus_player_open(
    const char* file_path,
    int32_t target_sample_rate
);

int32_t opus_player_read_pcm(
    PlayerDecoder* player,
    int16_t* out_pcm,
    size_t max_samples
);

int32_t opus_player_seek(
    PlayerDecoder* player,
    uint64_t position_ms
);

int64_t opus_player_get_duration_ms(PlayerDecoder* player);

int64_t opus_player_get_position_ms(PlayerDecoder* player);

int32_t opus_player_get_channels(PlayerDecoder* player);

int32_t opus_player_get_sample_rate(PlayerDecoder* player);

void opus_player_destroy(PlayerDecoder* player);

// File & Buffer conversions
int32_t opus_encode_file(
    const char* pcm_path,
    const char* ogg_path,
    int32_t sample_rate,
    int32_t channels,
    int32_t bitrate,
    int32_t application,
    uint64_t* out_duration_ms,
    uint64_t* out_file_size
);

int32_t opus_decode_file(
    const char* ogg_path,
    const char* pcm_path,
    int32_t target_sample_rate,
    int32_t* out_channels
);

int32_t opus_encode_buffer(
    const int16_t* pcm,
    size_t pcm_count,
    int32_t sample_rate,
    int32_t channels,
    int32_t bitrate,
    int32_t application,
    uint8_t** out_ptr,
    size_t* out_len
);

int32_t opus_decode_buffer(
    const uint8_t* ogg,
    size_t ogg_len,
    int32_t target_sample_rate,
    int16_t** out_pcm,
    size_t* out_count,
    int32_t* out_channels
);

void opus_free_buffer(uint8_t* ptr, size_t len);

void opus_free_pcm_buffer(int16_t* ptr, size_t count);

#ifdef __cplusplus
}
#endif

#endif /* EXPO_AUDIO_OPUS_H */
