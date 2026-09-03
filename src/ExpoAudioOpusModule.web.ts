import { registerWebModule, NativeModule } from "expo";
import type { ExpoAudioOpusEvents } from "./ExpoAudioOpus.types";

class ExpoAudioOpusWebModule extends NativeModule<ExpoAudioOpusEvents> {
  async requestPermissionsAsync() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async getPermissionsAsync() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async startRecording() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async pauseRecording() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async resumeRecording() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async stopRecording() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async startPlayback() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async pausePlayback() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async resumePlayback() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async stopPlayback() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async seekTo() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async encodeFile() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async decodeFile() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async encodePcmToOgg() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
  async decodeOggToPcm() {
    throw new Error("ExpoAudioOpus is not supported on web yet.");
  }
}

export default registerWebModule(ExpoAudioOpusWebModule, "ExpoAudioOpus");
