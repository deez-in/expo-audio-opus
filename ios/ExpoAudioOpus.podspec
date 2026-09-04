Pod::Spec.new do |s|
  s.name           = 'ExpoAudioOpus'
  s.version        = '0.2.0'
  s.summary        = 'Expo Native Module for opus-pure audio recording, playback, and codec'
  s.description    = 'Record and playback Ogg Opus audio with AVAudioEngine and convert PCM to Opus'
  s.author         = 'debarkamondal'
  s.homepage       = 'https://github.com/deez-in/expo-audio-opus'
  s.platforms      = {
    :ios => '15.1'
  }
  s.source         = { git: '' }
  s.static_framework = true

  s.dependency 'ExpoModulesCore'

  s.pod_target_xcconfig = {
    'DEFINES_MODULE' => 'YES',
  }

  s.source_files = "**/*.{h,m,mm,swift,hpp,cpp}"
  s.vendored_libraries = "rust/libexpo_audio_opus.a"
  s.public_header_files = "rust/expo_audio_opus.h"
end
