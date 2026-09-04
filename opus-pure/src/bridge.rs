//! Bridge layer providing streaming recording, streaming playback decoding,
//! and buffer/file conversion utilities over opus-pure.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom};
use std::ops::Range;

use opus_pure::ogg::{OggOpusReader, OggOpusWriter};
use opus_pure::{
    Application, Error, OpusDecoder, OpusEncoder, OpusHead, Result, Trim,
    MAX_PACKET_BYTES, MAX_PACKET_SAMPLES,
};

/// Result metadata after stopping a recording stream.
#[derive(Debug, Clone, Copy)]
pub struct RecordingInfo {
    pub duration_ms: u64,
    pub file_size_bytes: u64,
}

/// Live streaming recorder that receives PCM 16-bit audio chunks,
/// buffers them to 20ms Opus frames, encodes them, and writes
/// compliant Ogg Opus pages directly to disk.
pub struct RecorderStream {
    file_path: String,
    encoder: OpusEncoder,
    writer: Option<OggOpusWriter<BufWriter<File>>>,
    sample_rate: i32,
    channels: usize,
    frame_size: usize,
    pending_pcm: Vec<i16>,
    packet_buf: Vec<u8>,
    total_audio_samples: u64,
    pre_skip: u16,
}

impl RecorderStream {
    pub fn new(
        file_path: &str,
        sample_rate: i32,
        channels: usize,
        bitrate_bps: i32,
        application: Application,
    ) -> Result<Self> {
        let frame_size = (sample_rate / 50) as usize; // 20ms per packet
        let mut encoder = OpusEncoder::new(sample_rate, channels, application)?;
        encoder.bitrate_bps = bitrate_bps;

        let head = OpusHead::for_encoder(&encoder, sample_rate as u32);
        let pre_skip = head.pre_skip;

        let file = File::create(file_path)?;
        let writer = OggOpusWriter::new(BufWriter::new(file), head)?;

        Ok(Self {
            file_path: file_path.to_string(),
            encoder,
            writer: Some(writer),
            sample_rate,
            channels,
            frame_size,
            pending_pcm: Vec::with_capacity(frame_size * channels * 4),
            packet_buf: vec![0u8; MAX_PACKET_BYTES],
            total_audio_samples: 0,
            pre_skip,
        })
    }

    /// Appends incoming interleaved 16-bit PCM samples to the buffer and encodes
    /// complete 20ms frames.
    pub fn write_pcm(&mut self, pcm: &[i16]) -> Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or(Error::InvalidStream("recorder stream closed"))?;
        self.pending_pcm.extend_from_slice(pcm);

        let per_frame = self.frame_size * self.channels;
        while self.pending_pcm.len() >= per_frame {
            let block = &self.pending_pcm[..per_frame];
            let n = self
                .encoder
                .encode_s16(block, self.frame_size, &mut self.packet_buf)?;
            writer.write_packet(&self.packet_buf[..n])?;
            self.total_audio_samples += self.frame_size as u64;
            self.pending_pcm.drain(..per_frame);
        }
        Ok(())
    }

    /// Finishes recording: flushes encoder delay with padding silence,
    /// marks the final page with correct granule position and end-of-stream flag,
    /// and closes the file.
    pub fn finish(mut self) -> Result<RecordingInfo> {
        let mut writer = self
            .writer
            .take()
            .ok_or(Error::InvalidStream("recorder stream already finished"))?;
        let ticks = 48_000 / self.sample_rate as usize;
        let pending_samples = self.pending_pcm.len() / self.channels;
        let total_samples = self.total_audio_samples as usize + pending_samples;

        // Flushing frames: only flush pending samples plus pre-skip delay
        let delay_samples = (self.pre_skip as usize).div_ceil(ticks);
        let flush_samples = pending_samples + delay_samples;
        let flush_frames = flush_samples.div_ceil(self.frame_size);
        let final_granule = u64::from(self.pre_skip) + (total_samples * ticks) as u64;

        let per_frame = self.frame_size * self.channels;
        let mut block = vec![0i16; per_frame];

        for i in 0..flush_frames {
            let chunk_start = i * per_frame;
            if chunk_start < self.pending_pcm.len() {
                let available = (self.pending_pcm.len() - chunk_start).min(per_frame);
                block[..available]
                    .copy_from_slice(&self.pending_pcm[chunk_start..chunk_start + available]);
                block[available..].fill(0);
            } else {
                block.fill(0);
            }

            let n = self
                .encoder
                .encode_s16(&block, self.frame_size, &mut self.packet_buf)?;
            if i + 1 == flush_frames {
                let duration = final_granule.saturating_sub(writer.granule() as u64);
                writer.write_packet_with_duration(&self.packet_buf[..n], duration as u32)?;
            } else {
                writer.write_packet(&self.packet_buf[..n])?;
            }
        }

        writer.finish()?;

        let duration_ms = if self.sample_rate > 0 {
            (total_samples as u64 * 1000) / self.sample_rate as u64
        } else {
            0
        };

        let file_size_bytes = std::fs::metadata(&self.file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(RecordingInfo {
            duration_ms,
            file_size_bytes,
        })
    }
}

/// Live streaming decoder that opens an Ogg Opus file, parses headers,
/// and streams out 16-bit PCM samples on demand for native playback.
pub struct PlayerDecoder {
    file_path: String,
    reader: OggOpusReader<BufReader<File>>,
    decoder: OpusDecoder,
    trim: Trim,
    sample_rate: i32,
    channels: usize,
    block: Vec<i16>,
    pending: Range<usize>,
    duration_ms: u64,
    current_sample_pos: u64,
}

impl PlayerDecoder {
    pub fn open(file_path: &str, target_sample_rate: i32) -> Result<Self> {
        let file = File::open(file_path)?;
        let reader = OggOpusReader::new(BufReader::new(file))?;
        let channels = reader.head().channel_count as usize;

        let decoder = reader.head().decoder(target_sample_rate)?;
        let trim = Trim::new(reader.head(), target_sample_rate, channels)?;

        let duration_ms =
            detect_file_duration_ms(file_path, reader.head().pre_skip).unwrap_or(0);

        Ok(Self {
            file_path: file_path.to_string(),
            reader,
            decoder,
            trim,
            sample_rate: target_sample_rate,
            channels,
            block: vec![0i16; MAX_PACKET_SAMPLES * channels],
            pending: 0..0,
            duration_ms,
            current_sample_pos: 0,
        })
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    pub fn position_ms(&self) -> u64 {
        if self.sample_rate > 0 && self.channels > 0 {
            (self.current_sample_pos * 1000) / (self.sample_rate as u64 * self.channels as u64)
        } else {
            0
        }
    }

    /// Fills `out` with decoded interleaved 16-bit PCM samples.
    /// Returns the number of samples written into `out`. Returns 0 on EOF.
    pub fn read_pcm(&mut self, out: &mut [i16]) -> Result<usize> {
        let mut written = 0;
        while written < out.len() {
            while self.pending.is_empty() {
                let Some(packet) = self.reader.read_packet()? else {
                    return Ok(written);
                };
                let n = self
                    .decoder
                    .decode_s16(&packet.data, MAX_PACKET_SAMPLES, &mut self.block)?;
                self.pending = self.trim.keep_range(&packet, n * self.channels);
            }
            let take = (out.len() - written).min(self.pending.len());
            let src = self.pending.start..self.pending.start + take;
            out[written..written + take].copy_from_slice(&self.block[src]);
            self.pending.start += take;
            written += take;
            self.current_sample_pos += take as u64;
        }
        Ok(written)
    }

    /// Seeks to `target_ms` by rewinding the file and decoding forward.
    pub fn seek(&mut self, target_ms: u64) -> Result<()> {
        let target_samples = (target_ms * self.sample_rate as u64 * self.channels as u64) / 1000;

        let file = File::open(&self.file_path)?;
        self.reader = OggOpusReader::new(BufReader::new(file))?;
        let _ = self.decoder.reset_state();
        self.trim = Trim::new(self.reader.head(), self.sample_rate, self.channels)?;
        self.pending = 0..0;
        self.current_sample_pos = 0;

        let mut scratch = vec![0i16; 1024];
        let mut skipped = 0;
        while skipped < target_samples {
            let to_read = (target_samples - skipped).min(scratch.len() as u64) as usize;
            let n = self.read_pcm(&mut scratch[..to_read])?;
            if n == 0 {
                break;
            }
            skipped += n as u64;
        }
        Ok(())
    }
}

/// Helper function to detect duration of an Ogg Opus file from its trailing page.
fn detect_file_duration_ms(path: &str, pre_skip: u16) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len < 28 {
        return None;
    }

    let search_bytes = (len as usize).min(65536);
    file.seek(SeekFrom::End(-(search_bytes as i64))).ok()?;

    let mut buf = vec![0u8; search_bytes];
    file.read_exact(&mut buf).ok()?;

    // Search backwards for the last 'OggS' magic header
    for i in (0..buf.len().saturating_sub(14)).rev() {
        if &buf[i..i + 4] == b"OggS" {
            let granule = u64::from_le_bytes(buf[i + 6..i + 14].try_into().ok()?);
            if granule != u64::MAX && granule > u64::from(pre_skip) {
                let samples_48k = granule - u64::from(pre_skip);
                let ms = (samples_48k * 1000) / 48_000;
                return Some(ms);
            }
        }
    }
    None
}

/// In-memory: encodes raw 16-bit PCM slice into a compliant Ogg Opus container byte vector.
pub fn encode_pcm_to_ogg_buffer(
    pcm: &[i16],
    sample_rate: i32,
    channels: usize,
    bitrate_bps: i32,
    application: Application,
) -> Result<Vec<u8>> {
    let frame_size = (sample_rate / 50) as usize; // 20ms
    let mut encoder = OpusEncoder::new(sample_rate, channels, application)?;
    encoder.bitrate_bps = bitrate_bps;

    let head = OpusHead::for_encoder(&encoder, sample_rate as u32);
    let ticks = 48_000 / sample_rate as usize;
    let total_samples = pcm.len() / channels;

    let frames = (total_samples + (head.pre_skip as usize).div_ceil(ticks)).div_ceil(frame_size);
    let final_granule = u64::from(head.pre_skip) + (total_samples * ticks) as u64;

    let mut writer = OggOpusWriter::new(Vec::new(), head)?;
    let mut packet = vec![0u8; MAX_PACKET_BYTES];
    let per_frame = frame_size * channels;
    let mut block = vec![0i16; per_frame];

    for i in 0..frames {
        let start = (i * per_frame).min(pcm.len());
        let end = (start + per_frame).min(pcm.len());
        block[..end - start].copy_from_slice(&pcm[start..end]);
        block[end - start..].fill(0);

        let n = encoder.encode_s16(&block, frame_size, &mut packet)?;
        if i + 1 == frames {
            let duration = final_granule.saturating_sub(writer.granule() as u64);
            writer.write_packet_with_duration(&packet[..n], duration as u32)?;
        } else {
            writer.write_packet(&packet[..n])?;
        }
    }
    writer.finish()
}

/// In-memory: decodes an Ogg Opus container byte slice into interleaved 16-bit PCM.
pub fn decode_ogg_to_pcm_buffer(
    ogg_bytes: &[u8],
    target_sample_rate: i32,
) -> Result<(Vec<i16>, u32, u32)> {
    let mut reader = OggOpusReader::new(std::io::Cursor::new(ogg_bytes))?;
    let channels = reader.head().channel_count as usize;
    let mut decoder = reader.head().decoder(target_sample_rate)?;
    let mut trim = Trim::new(reader.head(), target_sample_rate, channels)?;

    let mut block = vec![0i16; MAX_PACKET_SAMPLES * channels];
    let mut pcm = Vec::new();

    while let Some(packet) = reader.read_packet()? {
        let n = decoder.decode_s16(&packet.data, MAX_PACKET_SAMPLES, &mut block)?;
        let range = trim.keep_range(&packet, n * channels);
        pcm.extend_from_slice(&block[range]);
    }
    Ok((pcm, target_sample_rate as u32, channels as u32))
}

/// Converts a raw 16-bit PCM file to an Ogg Opus file.
pub fn encode_pcm_file(
    pcm_path: &str,
    ogg_path: &str,
    sample_rate: i32,
    channels: usize,
    bitrate_bps: i32,
    application: Application,
) -> Result<RecordingInfo> {
    let mut file = File::open(pcm_path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    let pcm: Vec<i16> = bytes
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();

    let mut stream =
        RecorderStream::new(ogg_path, sample_rate, channels, bitrate_bps, application)?;
    stream.write_pcm(&pcm)?;
    stream.finish()
}

/// Converts an Ogg Opus file to a raw 16-bit PCM file.
pub fn decode_ogg_file(
    ogg_path: &str,
    pcm_path: &str,
    target_sample_rate: i32,
) -> Result<(u32, u32)> {
    let file = File::open(ogg_path)?;
    let mut reader = OggOpusReader::new(BufReader::new(file))?;
    let channels = reader.head().channel_count as usize;
    let mut decoder = reader.head().decoder(target_sample_rate)?;
    let mut trim = Trim::new(reader.head(), target_sample_rate, channels)?;

    let out_file = File::create(pcm_path)?;
    let mut writer = BufWriter::new(out_file);
    let mut block = vec![0i16; MAX_PACKET_SAMPLES * channels];

    use std::io::Write;
    while let Some(packet) = reader.read_packet()? {
        let n = decoder.decode_s16(&packet.data, MAX_PACKET_SAMPLES, &mut block)?;
        let range = trim.keep_range(&packet, n * channels);
        for &sample in &block[range] {
            writer.write_all(&sample.to_le_bytes())?;
        }
    }
    writer.flush()?;
    Ok((target_sample_rate as u32, channels as u32))
}
