//! C-FFI bindings for iOS (Swift / Objective-C) interop.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::bridge::{
    decode_ogg_file, decode_ogg_to_pcm_buffer, encode_pcm_file, encode_pcm_to_ogg_buffer,
    PlayerDecoder, RecorderStream,
};
use opus_pure::Application;

fn parse_app(app: i32) -> Application {
    match app {
        1 => Application::Voip,
        2 => Application::RestrictedLowDelay,
        _ => Application::Audio,
    }
}

// ---------------------------------------------------------------------------
// Recorder API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_recorder_create(
    file_path: *const c_char,
    sample_rate: i32,
    channels: i32,
    bitrate: i32,
    application: i32,
) -> *mut RecorderStream {
    if file_path.is_null() || channels <= 0 || sample_rate <= 0 {
        return std::ptr::null_mut();
    }
    let path = match unsafe { CStr::from_ptr(file_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let app = parse_app(application);
    match RecorderStream::new(path, sample_rate, channels as usize, bitrate, app) {
        Ok(stream) => Box::into_raw(Box::new(stream)),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_recorder_write_pcm(
    stream: *mut RecorderStream,
    pcm: *const i16,
    count: usize,
) -> i32 {
    if stream.is_null() || pcm.is_null() {
        return -1;
    }
    let s = unsafe { &mut *stream };
    let slice = unsafe { std::slice::from_raw_parts(pcm, count) };
    match s.write_pcm(slice) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_recorder_finish(
    stream: *mut RecorderStream,
    out_duration_ms: *mut u64,
    out_file_size: *mut u64,
) -> i32 {
    if stream.is_null() {
        return -1;
    }
    let s = unsafe { Box::from_raw(stream) };
    match s.finish() {
        Ok(info) => {
            if !out_duration_ms.is_null() {
                unsafe { *out_duration_ms = info.duration_ms };
            }
            if !out_file_size.is_null() {
                unsafe { *out_file_size = info.file_size_bytes };
            }
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_recorder_destroy(stream: *mut RecorderStream) {
    if !stream.is_null() {
        unsafe { drop(Box::from_raw(stream)) };
    }
}

// ---------------------------------------------------------------------------
// Player API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_player_open(
    file_path: *const c_char,
    target_sample_rate: i32,
) -> *mut PlayerDecoder {
    if file_path.is_null() || target_sample_rate <= 0 {
        return std::ptr::null_mut();
    }
    let path = match unsafe { CStr::from_ptr(file_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    match PlayerDecoder::open(path, target_sample_rate) {
        Ok(player) => Box::into_raw(Box::new(player)),
        Err(_) => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_player_read_pcm(
    player: *mut PlayerDecoder,
    out_pcm: *mut i16,
    max_samples: usize,
) -> i32 {
    if player.is_null() || out_pcm.is_null() || max_samples == 0 {
        return -1;
    }
    let p = unsafe { &mut *player };
    let slice = unsafe { std::slice::from_raw_parts_mut(out_pcm, max_samples) };
    match p.read_pcm(slice) {
        Ok(n) => n as i32,
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_player_seek(player: *mut PlayerDecoder, position_ms: u64) -> i32 {
    if player.is_null() {
        return -1;
    }
    let p = unsafe { &mut *player };
    match p.seek(position_ms) {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_player_get_duration_ms(player: *mut PlayerDecoder) -> i64 {
    if player.is_null() {
        return -1;
    }
    unsafe { (*player).duration_ms() as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_player_get_position_ms(player: *mut PlayerDecoder) -> i64 {
    if player.is_null() {
        return -1;
    }
    unsafe { (*player).position_ms() as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_player_get_channels(player: *mut PlayerDecoder) -> i32 {
    if player.is_null() {
        return -1;
    }
    unsafe { (*player).channels() as i32 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_player_get_sample_rate(player: *mut PlayerDecoder) -> i32 {
    if player.is_null() {
        return -1;
    }
    unsafe { (*player).sample_rate() }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_player_destroy(player: *mut PlayerDecoder) {
    if !player.is_null() {
        unsafe { drop(Box::from_raw(player)) };
    }
}

// ---------------------------------------------------------------------------
// File and Buffer Conversion API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_encode_file(
    pcm_path: *const c_char,
    ogg_path: *const c_char,
    sample_rate: i32,
    channels: i32,
    bitrate: i32,
    application: i32,
    out_duration_ms: *mut u64,
    out_file_size: *mut u64,
) -> i32 {
    if pcm_path.is_null() || ogg_path.is_null() || sample_rate <= 0 || channels <= 0 {
        return -1;
    }
    let pcm_str = match unsafe { CStr::from_ptr(pcm_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let ogg_str = match unsafe { CStr::from_ptr(ogg_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let app = parse_app(application);
    match encode_pcm_file(pcm_str, ogg_str, sample_rate, channels as usize, bitrate, app) {
        Ok(info) => {
            if !out_duration_ms.is_null() {
                unsafe { *out_duration_ms = info.duration_ms };
            }
            if !out_file_size.is_null() {
                unsafe { *out_file_size = info.file_size_bytes };
            }
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_decode_file(
    ogg_path: *const c_char,
    pcm_path: *const c_char,
    target_sample_rate: i32,
    out_channels: *mut i32,
) -> i32 {
    if ogg_path.is_null() || pcm_path.is_null() || target_sample_rate <= 0 {
        return -1;
    }
    let ogg_str = match unsafe { CStr::from_ptr(ogg_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let pcm_str = match unsafe { CStr::from_ptr(pcm_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match decode_ogg_file(ogg_str, pcm_str, target_sample_rate) {
        Ok((_rate, channels)) => {
            if !out_channels.is_null() {
                unsafe { *out_channels = channels as i32 };
            }
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_encode_buffer(
    pcm: *const i16,
    pcm_count: usize,
    sample_rate: i32,
    channels: i32,
    bitrate: i32,
    application: i32,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if pcm.is_null()
        || out_ptr.is_null()
        || out_len.is_null()
        || channels <= 0
        || sample_rate <= 0
    {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts(pcm, pcm_count) };
    let app = parse_app(application);

    match encode_pcm_to_ogg_buffer(slice, sample_rate, channels as usize, bitrate, app) {
        Ok(ogg) => {
            let mut buf = ogg.into_boxed_slice();
            unsafe {
                *out_len = buf.len();
                *out_ptr = buf.as_mut_ptr();
            }
            std::mem::forget(buf);
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_decode_buffer(
    ogg: *const u8,
    ogg_len: usize,
    target_sample_rate: i32,
    out_pcm: *mut *mut i16,
    out_count: *mut usize,
    out_channels: *mut i32,
) -> i32 {
    if ogg.is_null() || out_pcm.is_null() || out_count.is_null() || target_sample_rate <= 0 {
        return -1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ogg, ogg_len) };

    match decode_ogg_to_pcm_buffer(slice, target_sample_rate) {
        Ok((pcm, _rate, channels)) => {
            let mut buf = pcm.into_boxed_slice();
            unsafe {
                *out_count = buf.len();
                *out_pcm = buf.as_mut_ptr();
                if !out_channels.is_null() {
                    *out_channels = channels as i32;
                }
            }
            std::mem::forget(buf);
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, len))) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn opus_free_pcm_buffer(ptr: *mut i16, count: usize) {
    if !ptr.is_null() && count > 0 {
        unsafe { drop(Box::from_raw(std::slice::from_raw_parts_mut(ptr, count))) };
    }
}
