//! Expo Audio Opus native crate.
//!
//! Exposes streaming recording, streaming playback decoding, and buffer/file
//! conversions for React Native / Expo via C-FFI (iOS) and JNI (Android).

pub mod bridge;

#[cfg(feature = "ffi")]
pub mod ffi;

#[cfg(feature = "jni")]
pub mod jni;

#[cfg(test)]
mod tests {
    use super::bridge::*;
    use opus_pure::Application;

    #[test]
    fn test_pcm_to_ogg_buffer_roundtrip() {
        let sample_rate = 48000;
        let channels = 1;
        let frame_size = 960; // 20ms
        let pcm = vec![0i16; frame_size * 5]; // 100ms of silence

        let ogg = encode_pcm_to_ogg_buffer(
            &pcm,
            sample_rate,
            channels,
            24000,
            Application::Voip,
        ).expect("encoding should succeed");

        assert_eq!(&ogg[..4], b"OggS");

        let (decoded, out_rate, out_channels) = decode_ogg_to_pcm_buffer(&ogg, sample_rate)
            .expect("decoding should succeed");

        assert_eq!(out_rate, 48000);
        assert_eq!(out_channels, 1);
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_recorder_and_player_stream_file() {
        let sample_rate = 48000;
        let channels = 1;
        let frame_size = 960;
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_recorder.opus");
        let file_path_str = file_path.to_str().unwrap();

        let mut recorder = RecorderStream::new(
            file_path_str,
            sample_rate,
            channels,
            24000,
            Application::Voip,
        ).expect("recorder creation should succeed");

        // Write 3 frames (60ms) of audio
        let pcm = vec![100i16; frame_size * 3];
        recorder.write_pcm(&pcm).expect("write_pcm should succeed");

        let info = recorder.finish().expect("finish should succeed");
        assert!(info.file_size_bytes > 0);
        assert!(info.duration_ms >= 60);

        let mut player = PlayerDecoder::open(file_path_str, sample_rate)
            .expect("player open should succeed");

        assert_eq!(player.channels(), 1);
        assert_eq!(player.sample_rate(), 48000);

        let mut out_pcm = vec![0i16; frame_size * 10];
        let n = player.read_pcm(&mut out_pcm).expect("read_pcm should succeed");
        assert!(n > 0);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_duration_check() {
        use super::bridge::*;
        use opus_pure::Application;
        let sample_rate = 48000;
        let channels = 1;
        let frame_size = 960;
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_duration.opus");
        let file_path_str = file_path.to_str().unwrap();

        let mut recorder = RecorderStream::new(
            file_path_str,
            sample_rate,
            channels,
            24000,
            Application::Voip,
        ).unwrap();

        // 10 frames = 200ms
        let pcm = vec![100i16; frame_size * 10];
        recorder.write_pcm(&pcm).unwrap();
        let info = recorder.finish().unwrap();

        let mut player = PlayerDecoder::open(file_path_str, sample_rate).unwrap();
        println!("Reported duration: {} ms, Detected file duration: {} ms", info.duration_ms, player.duration_ms());

        let mut total_read = 0;
        let mut buf = vec![0i16; 960];
        while let Ok(n) = player.read_pcm(&mut buf) {
            if n == 0 { break; }
            total_read += n;
        }
        println!("Total samples read: {}, Expected samples: {}", total_read, pcm.len());
        let _ = std::fs::remove_file(file_path);
    }

}
