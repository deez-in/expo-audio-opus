# AGENTS.md — expo-audio-opus

This document provides instructions and context for AI coding agents working in this repository.

## Ecosystem Context

> **This module wraps `opus-pure` (Rust) for React Native / Expo.** It provides native C-FFI (iOS) and JNI (Android) bindings so applications (such as `deezchatz-mobile`) can record voice notes directly to `.ogg` (Opus) files using hardware audio engines (`AVAudioEngine` on iOS, `AudioRecord` on Android), play them back (`AVAudioEngine` on iOS, `AudioTrack` on Android), and perform fast bidirectional PCM ↔ Ogg Opus conversions at native speeds.

```
deezchatz-mobile  →  ⭐ expo-audio-opus (this module)  →  opus-pure (pure Rust codec)
```

| Relationship | Details |
|-------------|---------|
| **Depends on** | `opus-pure` — included as an untouched git submodule at `opus-pure/` |
| **Used by** | `deezchatz-mobile` — for voice message recording, preview playback, and Opus encoding |
| **Wraps** | `opus-pure` via a local bridge crate at `rust/` |
| **Submodule Policy** | **DO NOT make changes inside `opus-pure/`!** All FFI, JNI, and bridge additions live in `rust/`. |

---

## Commands

```bash
# Compile TypeScript declarations and build distribution bundle
bun run prepare

# Platform-specific native builds
bun run build:android          # Cross-compile Rust for Android targets (arm64-v8a, armeabi-v7a, x86, x86_64)
bun run build:ios              # Cross-compile Rust for iOS device (aarch64-apple-ios)
bun run build:ios-sim          # Cross-compile Rust for iOS simulator (aarch64-apple-ios-sim)

# Run Rust tests
cd rust && cargo test --features=ffi,jni
```

---

## Project Structure

```
src/
  ExpoAudioOpusModule.ts       # TypeScript native module declaration (all method signatures)
  ExpoAudioOpus.types.ts       # TypeScript types (RecordingOptions, PlaybackStatus, etc.)
  ExpoAudioOpusModule.web.ts   # Web stub
  index.ts                     # Module entry point and event listener helpers
index.ts                       # Package root entry point

ios/
  ExpoAudioOpusModule.swift    # Swift Expo Module (AVAudioEngine record & play + C-FFI)
  ExpoAudioOpus.podspec        # CocoaPods spec (links libexpo_audio_opus.a)
  rust/                        # Pre-built static library and C headers

android/
  build.gradle                 # Gradle library configuration
  src/main/
    AndroidManifest.xml        # RECORD_AUDIO permission
    java/expo/modules/audioopus/ # Kotlin JNI wrapper (AudioRecord & AudioTrack)
    jniLibs/                   # Pre-built shared libraries (.so) for Android targets

opus-pure/                     # Pure-Rust Opus codec (Git submodule — KEEP CLEAN)

rust/                          # Native bridge crate connecting opus-pure to FFI & JNI
  Cargo.toml                   # crate-type = ["staticlib", "cdylib", "lib"]
  expo_audio_opus.h            # C header for iOS C-FFI
  src/
    lib.rs                     # Library declarations and unit tests
    bridge.rs                  # Streaming recorder, player decoder, file & buffer converters
    ffi.rs                     # C-FFI exports (#[unsafe(no_mangle)]) for iOS
    jni.rs                     # JNI exports (#[unsafe(no_mangle)]) for Android

scripts/
  cargo-android.ts             # Android cross-compilation pipeline
  cargo-ios.ts                 # iOS cross-compilation pipeline

expo-module.config.json        # Expo module registration config
```

---

## Architecture Boundaries

The architecture consists of four synchronized layers:

```
TypeScript declarations  ←→  Swift / Kotlin native modules  ←→  Rust extern "C" / JNI  ←→  opus-pure core
```

1. **Rust Core & Bridge** (`rust/src/`):
   - `RecorderStream`: Receives PCM 16-bit chunks, buffers into 20ms frames, encodes using `OpusEncoder`, and writes valid Ogg Opus pages with end-trim and granule positioning.
   - `PlayerDecoder`: Demuxes Ogg Opus container, decodes packets with `Trim` (accounting for pre-skip and end-trim), and streams PCM 16-bit to native audio outputs.
   - `ffi.rs`: C-FFI exports with `#[unsafe(no_mangle)]` for iOS.
   - `jni.rs`: JNI exports with `#[unsafe(no_mangle)]` under `Java_expo_modules_audioopus_ExpoAudioOpusModule_...` for Android.

2. **iOS Layer** (`ios/ExpoAudioOpusModule.swift`):
   - **Recording**: Captures mic input via `AVAudioEngine.inputNode.installTap`, converts to target format using `AVAudioConverter`, computes RMS amplitude for live metering, and writes to `RecorderStream`.
   - **Playback**: Feeds decoded PCM buffers from `PlayerDecoder` into `AVAudioPlayerNode` connected to `AVAudioEngine.mainMixerNode`.
   - **Permissions**: Manages `AVAudioSession.recordPermission`.

3. **Android Layer** (`android/src/main/java/expo/modules/audioopus/ExpoAudioOpusModule.kt`):
   - **Recording**: Uses `AudioRecord` on a dedicated background thread, computes amplitude, emits metering events, and writes PCM chunks to `recorderWritePcm`.
   - **Playback**: Uses `AudioTrack` on a dedicated worker thread streaming decoded PCM from `playerReadPcm`.
   - **Permissions**: Manages `RECORD_AUDIO` via Expo Permissions API.

4. **TypeScript Layer** (`src/ExpoAudioOpusModule.ts`, `src/ExpoAudioOpus.types.ts`, `index.ts`):
   - Exports the typed native module and event subscription helpers:
     - `addRecordingMeteringListener((event: MeteringEvent) => void)`
     - `addPlaybackStatusListener((status: PlaybackStatus) => void)`

---

## Memory Management Rules

- Stream handles (`RecorderStream`, `PlayerDecoder`) live on the **Rust heap** as raw pointers (`*mut RecorderStream`, `*mut PlayerDecoder`).
- iOS Swift manages pointers as `OpaquePointer`.
- Android Kotlin manages pointers as `Long` values.
- Finishing or stopping a stream calls `finish` / `destroy`, which reconstructs `Box::from_raw(...)` to properly deallocate Rust heap memory.
- In-memory buffers allocated by Rust during `encode_buffer` / `decode_buffer` are freed via `opus_free_buffer` / `opus_free_pcm_buffer`.

---

## Submodule Rule

**NEVER modify files inside `opus-pure/` directly.** The `opus-pure/` directory is an external submodule tracked at `fix/sec-and-perf`. All custom logic, JNI functions, C-FFI bindings, and streaming wrappers must reside exclusively in `rust/`.
