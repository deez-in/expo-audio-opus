import { NativeModule, requireNativeModule } from "expo";
import type {
  CodecOptions,
  DecodedPcmResult,
  ExpoAudioOpusEvents,
  FileConversionResult,
  FileDecodeResult,
  PlaybackInfo,
  PermissionResponse,
  RecordingOptions,
  RecordingResult,
} from "./ExpoAudioOpus.types";

declare class ExpoAudioOpusModule extends NativeModule<ExpoAudioOpusEvents> {
  // Permissions
  requestPermissionsAsync(): Promise<PermissionResponse>;
  getPermissionsAsync(): Promise<PermissionResponse>;

  // Recording
  startRecording(options?: RecordingOptions): Promise<void>;
  pauseRecording(): Promise<void>;
  resumeRecording(): Promise<void>;
  stopRecording(): Promise<RecordingResult>;

  // Playback
  startPlayback(uri: string): Promise<PlaybackInfo>;
  pausePlayback(): Promise<void>;
  resumePlayback(): Promise<void>;
  stopPlayback(): Promise<void>;
  seekTo(positionMs: number): Promise<void>;

  // Codec
  encodeFile(
    pcmPath: string,
    oggPath: string,
    options?: CodecOptions
  ): Promise<FileConversionResult>;
  decodeFile(
    oggPath: string,
    pcmPath: string,
    targetSampleRate?: number
  ): Promise<FileDecodeResult>;
  encodePcmToOgg(
    pcmData: Uint8Array,
    options?: CodecOptions
  ): Promise<Uint8Array>;
  decodeOggToPcm(
    oggData: Uint8Array,
    targetSampleRate?: number
  ): Promise<DecodedPcmResult>;
}

export default requireNativeModule<ExpoAudioOpusModule>("ExpoAudioOpus");
