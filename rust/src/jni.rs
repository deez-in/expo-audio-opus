//! JNI bindings for Android Kotlin integration.

use jni::objects::{JByteArray, JClass, JShortArray, JString};
use jni::sys::{jbyteArray, jint, jlong, jlongArray, jshortArray};
use jni::JNIEnv;

use crate::bridge::{
    decode_ogg_file, decode_ogg_to_pcm_buffer, encode_pcm_file, encode_pcm_to_ogg_buffer,
    PlayerDecoder, RecorderStream,
};
use opus_pure::Application;

fn parse_app(app: jint) -> Application {
    match app {
        1 => Application::Voip,
        2 => Application::RestrictedLowDelay,
        _ => Application::Audio,
    }
}

// ---------------------------------------------------------------------------
// Recorder JNI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_recorderCreate(
    mut env: JNIEnv,
    _: JClass,
    file_path: JString,
    sample_rate: jint,
    channels: jint,
    bitrate: jint,
    application: jint,
) -> jlong {
    let path: String = match env.get_string(&file_path) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };

    let app = parse_app(application);
    match RecorderStream::new(&path, sample_rate, channels as usize, bitrate, app) {
        Ok(stream) => Box::into_raw(Box::new(stream)) as jlong,
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_recorderWritePcm(
    env: JNIEnv,
    _: JClass,
    state_ptr: jlong,
    pcm: JShortArray,
) -> jint {
    if state_ptr == 0 {
        return -1;
    }
    let stream = unsafe { &mut *(state_ptr as *mut RecorderStream) };

    let len = match env.get_array_length(&pcm) {
        Ok(l) => l as usize,
        Err(_) => return -1,
    };

    let mut buf = vec![0i16; len];
    if env.get_short_array_region(&pcm, 0, &mut buf).is_err() {
        return -1;
    }

    match stream.write_pcm(&buf) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_recorderFinish(
    env: JNIEnv,
    _: JClass,
    state_ptr: jlong,
) -> jlongArray {
    if state_ptr == 0 {
        return std::ptr::null_mut();
    }
    let stream = unsafe { Box::from_raw(state_ptr as *mut RecorderStream) };
    match stream.finish() {
        Ok(info) => {
            let res = [info.duration_ms as i64, info.file_size_bytes as i64];
            match env.new_long_array(2) {
                Ok(arr) => {
                    let _ = env.set_long_array_region(&arr, 0, &res);
                    arr.into_raw()
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_recorderDestroy(
    _: JNIEnv,
    _: JClass,
    state_ptr: jlong,
) {
    if state_ptr != 0 {
        unsafe { drop(Box::from_raw(state_ptr as *mut RecorderStream)) };
    }
}

// ---------------------------------------------------------------------------
// Player JNI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_playerOpen(
    mut env: JNIEnv,
    _: JClass,
    file_path: JString,
    target_sample_rate: jint,
) -> jlong {
    let path: String = match env.get_string(&file_path) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };

    match PlayerDecoder::open(&path, target_sample_rate) {
        Ok(player) => Box::into_raw(Box::new(player)) as jlong,
        Err(_) => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_playerReadPcm(
    env: JNIEnv,
    _: JClass,
    state_ptr: jlong,
    out_pcm: JShortArray,
) -> jint {
    if state_ptr == 0 {
        return -1;
    }
    let player = unsafe { &mut *(state_ptr as *mut PlayerDecoder) };

    let len = match env.get_array_length(&out_pcm) {
        Ok(l) => l as usize,
        Err(_) => return -1,
    };

    let mut buf = vec![0i16; len];
    match player.read_pcm(&mut buf) {
        Ok(n) => {
            if n > 0 {
                let _ = env.set_short_array_region(&out_pcm, 0, &buf[..n]);
            }
            n as jint
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_playerSeek(
    _: JNIEnv,
    _: JClass,
    state_ptr: jlong,
    position_ms: jlong,
) -> jint {
    if state_ptr == 0 {
        return -1;
    }
    let player = unsafe { &mut *(state_ptr as *mut PlayerDecoder) };
    match player.seek(position_ms as u64) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_playerGetDurationMs(
    _: JNIEnv,
    _: JClass,
    state_ptr: jlong,
) -> jlong {
    if state_ptr == 0 {
        return -1;
    }
    let player = unsafe { &*(state_ptr as *mut PlayerDecoder) };
    player.duration_ms() as jlong
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_playerGetPositionMs(
    _: JNIEnv,
    _: JClass,
    state_ptr: jlong,
) -> jlong {
    if state_ptr == 0 {
        return -1;
    }
    let player = unsafe { &*(state_ptr as *mut PlayerDecoder) };
    player.position_ms() as jlong
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_playerGetChannels(
    _: JNIEnv,
    _: JClass,
    state_ptr: jlong,
) -> jint {
    if state_ptr == 0 {
        return -1;
    }
    let player = unsafe { &*(state_ptr as *mut PlayerDecoder) };
    player.channels() as jint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_playerGetSampleRate(
    _: JNIEnv,
    _: JClass,
    state_ptr: jlong,
) -> jint {
    if state_ptr == 0 {
        return -1;
    }
    let player = unsafe { &*(state_ptr as *mut PlayerDecoder) };
    player.sample_rate()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_playerDestroy(
    _: JNIEnv,
    _: JClass,
    state_ptr: jlong,
) {
    if state_ptr != 0 {
        unsafe { drop(Box::from_raw(state_ptr as *mut PlayerDecoder)) };
    }
}

// ---------------------------------------------------------------------------
// File & Buffer Conversion JNI
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_encodeFile(
    mut env: JNIEnv,
    _: JClass,
    pcm_path: JString,
    ogg_path: JString,
    sample_rate: jint,
    channels: jint,
    bitrate: jint,
    application: jint,
) -> jlongArray {
    let pcm_str: String = match env.get_string(&pcm_path) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };
    let ogg_str: String = match env.get_string(&ogg_path) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let app = parse_app(application);
    match encode_pcm_file(&pcm_str, &ogg_str, sample_rate, channels as usize, bitrate, app) {
        Ok(info) => {
            let res = [info.duration_ms as i64, info.file_size_bytes as i64];
            match env.new_long_array(2) {
                Ok(arr) => {
                    let _ = env.set_long_array_region(&arr, 0, &res);
                    arr.into_raw()
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_decodeFile(
    mut env: JNIEnv,
    _: JClass,
    ogg_path: JString,
    pcm_path: JString,
    target_sample_rate: jint,
) -> jint {
    let ogg_str: String = match env.get_string(&ogg_path) {
        Ok(s) => s.into(),
        Err(_) => return -1,
    };
    let pcm_str: String = match env.get_string(&pcm_path) {
        Ok(s) => s.into(),
        Err(_) => return -1,
    };

    match decode_ogg_file(&ogg_str, &pcm_str, target_sample_rate) {
        Ok((_rate, channels)) => channels as jint,
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_encodeBuffer(
    env: JNIEnv,
    _: JClass,
    pcm: JShortArray,
    sample_rate: jint,
    channels: jint,
    bitrate: jint,
    application: jint,
) -> jbyteArray {
    let len = match env.get_array_length(&pcm) {
        Ok(l) => l as usize,
        Err(_) => return std::ptr::null_mut(),
    };

    let mut buf = vec![0i16; len];
    if env.get_short_array_region(&pcm, 0, &mut buf).is_err() {
        return std::ptr::null_mut();
    }

    let app = parse_app(application);
    match encode_pcm_to_ogg_buffer(&buf, sample_rate, channels as usize, bitrate, app) {
        Ok(ogg) => match env.byte_array_from_slice(&ogg) {
            Ok(arr) => arr.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_expo_modules_audioopus_ExpoAudioOpusModule_decodeBuffer(
    env: JNIEnv,
    _: JClass,
    ogg: JByteArray,
    target_sample_rate: jint,
) -> jshortArray {
    let len = match env.get_array_length(&ogg) {
        Ok(l) => l as usize,
        Err(_) => return std::ptr::null_mut(),
    };

    let mut buf = vec![0u8; len];
    let i8_buf = unsafe { std::slice::from_raw_parts_mut(buf.as_mut_ptr() as *mut i8, len) };
    if env.get_byte_array_region(&ogg, 0, i8_buf).is_err() {
        return std::ptr::null_mut();
    }

    match decode_ogg_to_pcm_buffer(&buf, target_sample_rate) {
        Ok((pcm, _rate, _channels)) => {
            match env.new_short_array(pcm.len() as i32) {
                Ok(arr) => {
                    let _ = env.set_short_array_region(&arr, 0, &pcm);
                    arr.into_raw()
                }
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}
