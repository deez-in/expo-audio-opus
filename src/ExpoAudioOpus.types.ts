export interface RecordingOptions {
  sampleRate?: 48000 | 24000 | 16000 | 12000 | 8000;
  channels?: 1 | 2;
  bitrate?: number;
  /**
   * Opus application mode:
   * 1: Voip (speech optimized, default)
   * 0: Audio (general music/mixed)
   * 2: RestrictedLowDelay (lowest latency)
   */
  application?: 0 | 1 | 2;
  enableMetering?: boolean;
}

export interface RecordingResult {
  uri: string;
  durationMs: number;
  fileSize: number;
}

export interface PlaybackStatus {
  isPlaying: boolean;
  isPaused: boolean;
  positionMs: number;
  durationMs: number;
  didJustFinish: boolean;
}

export interface PlaybackInfo {
  durationMs: number;
  channels: number;
  sampleRate: number;
}

export interface CodecOptions {
  sampleRate?: number;
  channels?: number;
  bitrate?: number;
  application?: 0 | 1 | 2;
}

export interface DecodedPcmResult {
  pcm: Uint8Array;
  sampleRate: number;
  channels: number;
}

export interface FileConversionResult {
  uri: string;
  durationMs: number;
  fileSize: number;
}

export interface FileDecodeResult {
  sampleRate: number;
  channels: number;
}

export interface MeteringEvent {
  amplitude: number; // 0.0 to 1.0 normalized
}

export interface PermissionResponse {
  status: "granted" | "denied" | "undetermined";
  granted: boolean;
  canAskAgain: boolean;
}

export type ExpoAudioOpusEvents = {
  onRecordingMetering: (event: MeteringEvent) => void;
  onPlaybackStatusUpdate: (status: PlaybackStatus) => void;
};
