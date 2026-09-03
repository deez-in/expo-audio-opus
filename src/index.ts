import type { EventSubscription } from "expo-modules-core";
import ExpoAudioOpusModule from "./ExpoAudioOpusModule";
import type {
  MeteringEvent,
  PlaybackStatus,
} from "./ExpoAudioOpus.types";

export function addRecordingMeteringListener(
  listener: (event: MeteringEvent) => void
): EventSubscription {
  return ExpoAudioOpusModule.addListener("onRecordingMetering", listener);
}

export function addPlaybackStatusListener(
  listener: (status: PlaybackStatus) => void
): EventSubscription {
  return ExpoAudioOpusModule.addListener("onPlaybackStatusUpdate", listener);
}

export { default } from "./ExpoAudioOpusModule";
export * from "./ExpoAudioOpus.types";
