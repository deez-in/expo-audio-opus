# Changelog

Notable changes to `expo-audio-opus`, newest first.

## 0.2.0 — 2026-09-04

### Fixed
- **Recorder Flush Frame Calculation**: Corrected the flush frame calculation during `RecorderStream::finish` in Rust bridge. It now flushes only pending buffered samples plus pre-skip delay rather than total accumulated audio samples, ensuring accurate recording durations without extraneous trailing silence or misaligned granules.
- **Android Thread Safety**: 
  - Stopped `AudioRecord` before attempting to join the recording worker thread to ensure timely teardown.
  - Added a thread check before calling `join()` on `playbackThread` in `ExpoAudioOpusModule.kt`, preventing self-join deadlocks when playback naturally completes from within the worker thread.

### Changed
- **Upstream Codec Synchronization**: Updated the `opus-pure` submodule to the latest upstream release, bringing in improvements to CELT/SILK analysis, range coding, repacketizer handling, and gapless audio trimming.
- **Prebuilt Android Binaries**: Recompiled and updated Android native shared libraries (`libexpo_audio_opus.so`) across all supported architectures (`arm64-v8a`, `armeabi-v7a`, `x86_64`).
- **CI/CD Pipeline**: Added GitHub Actions workflow (`.github/workflows/publish.yml`) on `macos-latest` to automate building native Android & iOS libraries and publishing npm releases with provenance.
- **Repository Metadata**: Corrected repository, issue tracker, and homepage URLs to `deez-in/expo-audio-opus`.

---

## 0.1.0 — 2026-09-03

- Initial release.
- Hardware-accelerated Ogg Opus recording and playback (`AVAudioEngine` on iOS, `AudioRecord`/`AudioTrack` on Android).
- In-memory bidirectional PCM ↔ Ogg Opus conversions.
- Native Rust bridge powered by `opus-pure`.
