# expo-audio-opus

High-performance, pure-Rust Opus audio codec wrapper for **Expo** and **React Native**, powered by [`opus-pure`](https://github.com/stephenberry/opus-pure).

Provides native audio recording directly to standard `.ogg` Opus files (`AVAudioEngine` on iOS, `AudioRecord` on Android), live audio playback (`AVAudioEngine` on iOS, `AudioTrack` on Android), real-time metering for live waveforms, and fast bidirectional PCM ↔ Ogg Opus conversions.

---

## Features

- 🎙️ **Hardware-Accelerated Recording**: Captures audio using `AVAudioEngine` (iOS) and `AudioRecord` (Android), encoding on the fly to `.ogg` (Opus) files.
- 🔊 **Native Playback**: Streams and decodes `.ogg` Opus audio through `AVAudioEngine` (iOS) and `AudioTrack` (Android) with play, pause, resume, seek, and status events.
- 📊 **Real-time Waveform Metering**: Emits normalized amplitude (0.0 to 1.0) while recording for animated waveform visualization.
- 🔄 **Bidirectional Codec**: Convert raw 16-bit PCM buffers or files to `.ogg` Opus containers and vice versa.
- 🦀 **Pure Rust Engine**: Built on `opus-pure` with zero C dependencies or external shared library bloat.
- 📱 **Clean Expo Module API**: Fully typed TypeScript interface with event listeners.

---

## Installation

```bash
npm install expo-audio-opus
# or
bun add expo-audio-opus
# or
yarn add expo-audio-opus
```

### Permissions Configuration

#### iOS (`app.json` / `Info.plist`)
Add the `NSMicrophoneUsageDescription` to your `app.json`:

```json
{
  "expo": {
    "plugins": [
      [
        "expo-audio-opus",
        {
          "microphonePermission": "Allow $(PRODUCT_NAME) to access the microphone for voice messages."
        }
      ]
    ]
  }
}
```

#### Android (`AndroidManifest.xml`)
The module automatically includes the required permissions:
```xml
<uses-permission android:name="android.permission.RECORD_AUDIO" />
```

---

## Quick Start

### 1. Recording a Voice Note

```typescript
import ExpoAudioOpus, {
  addRecordingMeteringListener,
  RecordingResult,
} from "expo-audio-opus";

// 1. Request microphone permissions
const { granted } = await ExpoAudioOpus.requestPermissionsAsync();
if (!granted) {
  alert("Microphone permission required");
  return;
}

// 2. Subscribe to real-time audio amplitude for waveforms
const meteringSubscription = addRecordingMeteringListener(({ amplitude }) => {
  console.log("Current audio level (0.0 to 1.0):", amplitude);
});

// 3. Start recording
await ExpoAudioOpus.startRecording({
  sampleRate: 48000,   // 48kHz (standard Opus voice)
  channels: 1,        // Mono
  bitrate: 24000,     // 24 kbps
  application: 1,     // 1: Voip (speech), 0: Audio (music), 2: LowDelay
  enableMetering: true,
});

// ... recording in progress ...

// 4. Stop recording
const result: RecordingResult = await ExpoAudioOpus.stopRecording();
meteringSubscription.remove();

console.log("Recorded file URI:", result.uri);
console.log("Duration (ms):", result.durationMs);
console.log("File size (bytes):", result.fileSize);
```

---

### 2. Audio Playback

```typescript
import ExpoAudioOpus, {
  addPlaybackStatusListener,
} from "expo-audio-opus";

// 1. Listen for playback progress
const statusSubscription = addPlaybackStatusListener((status) => {
  console.log(`Position: ${status.positionMs}ms / ${status.durationMs}ms`);
  if (status.didJustFinish) {
    console.log("Playback completed!");
  }
});

// 2. Start playback
const info = await ExpoAudioOpus.startPlayback(result.uri);
console.log("Playing audio of duration:", info.durationMs);

// Playback controls
await ExpoAudioOpus.pausePlayback();
await ExpoAudioOpus.resumePlayback();
await ExpoAudioOpus.seekTo(2500); // Seek to 2.5 seconds
await ExpoAudioOpus.stopPlayback();

statusSubscription.remove();
```

---

### 3. Standalone File Conversion (PCM ↔ Ogg Opus)

```typescript
import ExpoAudioOpus from "expo-audio-opus";

// Convert a raw PCM 16-bit file to an Ogg Opus file
const encoded = await ExpoAudioOpus.encodeFile(
  "file:///path/to/input.pcm",
  "file:///path/to/output.ogg",
  {
    sampleRate: 48000,
    channels: 1,
    bitrate: 24000,
  }
);
console.log("Encoded Ogg file:", encoded.uri, encoded.durationMs);

// Decode an Ogg Opus file back to raw PCM 16-bit file
const decoded = await ExpoAudioOpus.decodeFile(
  "file:///path/to/output.ogg",
  "file:///path/to/decoded.pcm",
  48000 // target sample rate
);
console.log("Decoded PCM channels:", decoded.channels);
```

---

### 4. In-Memory Buffer Conversion

```typescript
import ExpoAudioOpus from "expo-audio-opus";

const pcmData = new Uint8Array([...]); // 16-bit interleaved PCM samples

// Encode PCM buffer to Ogg Opus bytes
const oggBytes: Uint8Array = await ExpoAudioOpus.encodePcmToOgg(pcmData, {
  sampleRate: 48000,
  channels: 1,
  bitrate: 24000,
});

// Decode Ogg Opus bytes back to PCM
const { pcm, sampleRate, channels } = await ExpoAudioOpus.decodeOggToPcm(oggBytes, 48000);
console.log("Decoded PCM bytes length:", pcm.length);
```

---

## API Reference

### Methods

| Method | Parameters | Return Type | Description |
|---|---|---|---|
| `requestPermissionsAsync()` | None | `Promise<PermissionResponse>` | Requests microphone permission |
| `getPermissionsAsync()` | None | `Promise<PermissionResponse>` | Checks microphone permission status |
| `startRecording(options?)` | `RecordingOptions` | `Promise<void>` | Starts recording to a new `.opus` file |
| `pauseRecording()` | None | `Promise<void>` | Pauses recording |
| `resumeRecording()` | None | `Promise<void>` | Resumes paused recording |
| `stopRecording()` | None | `Promise<RecordingResult>` | Stops recording and finalizes `.ogg` file |
| `startPlayback(uri)` | `uri: string` | `Promise<PlaybackInfo>` | Opens and starts playing an Opus audio file |
| `pausePlayback()` | None | `Promise<void>` | Pauses playback |
| `resumePlayback()` | None | `Promise<void>` | Resumes playback |
| `stopPlayback()` | None | `Promise<void>` | Stops playback and releases audio resources |
| `seekTo(positionMs)` | `positionMs: number` | `Promise<void>` | Seeks to target timestamp in milliseconds |
| `encodeFile(pcmPath, oggPath, options?)` | `pcmPath, oggPath, options` | `Promise<FileConversionResult>` | Encodes a PCM file to an Ogg Opus file |
| `decodeFile(oggPath, pcmPath, targetRate?)` | `oggPath, pcmPath, rate` | `Promise<FileDecodeResult>` | Decodes an Ogg Opus file to a PCM file |
| `encodePcmToOgg(pcmData, options?)` | `pcmData, options` | `Promise<Uint8Array>` | Encodes PCM byte buffer to Ogg Opus buffer |
| `decodeOggToPcm(oggData, targetRate?)` | `oggData, rate` | `Promise<DecodedPcmResult>` | Decodes Ogg Opus buffer to PCM bytes |

---

### Events

- **`addRecordingMeteringListener((event: { amplitude: number }) => void)`**: Emits live normalized audio amplitude (0.0 to 1.0) approximately every 50ms while recording.
- **`addPlaybackStatusListener((status: PlaybackStatus) => void)`**: Emits current playback position, duration, and finish state approximately every 100ms.

---

## Building from Source

To compile the native Rust libraries:

```bash
# Clone with submodules
git clone --recurse-submodules https://github.com/debarkamondal/expo-audio-opus.git

# Install dependencies
bun install

# Compile TypeScript declarations
bun run prepare

# Cross-compile for Android (requires Android NDK)
bun run build:android

# Cross-compile for iOS (requires Xcode and iOS rust targets)
bun run build:ios
bun run build:ios-sim
```

---

## License

MIT © [debarkamondal](https://github.com/debarkamondal)
