# Keep ExpoAudioOpusModule to prevent JNI methods from being renamed or stripped
-keep class expo.modules.audioopus.ExpoAudioOpusModule {
    *;
}
