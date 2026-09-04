package expo.modules.audioopus

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioRecord
import android.media.AudioTrack
import android.media.MediaRecorder
import androidx.core.content.ContextCompat
import expo.modules.interfaces.permissions.Permissions
import expo.modules.kotlin.Promise
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition
import java.io.File
import java.util.UUID
import kotlin.concurrent.thread
import kotlin.math.max
import kotlin.math.min
import kotlin.math.sqrt

class ExpoAudioOpusModule : Module() {
  private val context: Context
    get() = appContext.reactContext ?: throw Exception("React context is unavailable")

  // Recorder state
  private var recorderStreamPtr: Long = 0
  private var audioRecord: AudioRecord? = null
  private var isRecording = false
  private var isRecordingPaused = false
  private var recordingThread: Thread? = null
  private var recordingFilePath: String? = null
  private var currentAmplitude: Float = 0f

  // Playback state
  private var playerStreamPtr: Long = 0
  private var audioTrack: AudioTrack? = null
  private var isPlaying = false
  private var isPlaybackPaused = false
  private var playbackThread: Thread? = null
  private var playbackDurationMs: Long = 0

  override fun definition() = ModuleDefinition {
    Name("ExpoAudioOpus")

    Events("onRecordingMetering", "onPlaybackStatusUpdate")

    // -------------------------------------------------------------------
    // Permissions
    // -------------------------------------------------------------------
    AsyncFunction("requestPermissionsAsync") { promise: Promise ->
      Permissions.askForPermissionsWithPermissionsManager(
        appContext.permissions,
        promise,
        Manifest.permission.RECORD_AUDIO
      )
    }

    AsyncFunction("getPermissionsAsync") { promise: Promise ->
      Permissions.getPermissionsWithPermissionsManager(
        appContext.permissions,
        promise,
        Manifest.permission.RECORD_AUDIO
      )
    }

    // -------------------------------------------------------------------
    // Recording (AudioRecord)
    // -------------------------------------------------------------------
    AsyncFunction("startRecording") { options: Map<String, Any>? ->
      if (isRecording) {
        throw Exception("Already recording")
      }

      val permissionCheck = ContextCompat.checkSelfPermission(
        context,
        Manifest.permission.RECORD_AUDIO
      )
      if (permissionCheck != PackageManager.PERMISSION_GRANTED) {
        throw Exception("RECORD_AUDIO permission not granted")
      }

      val sampleRate = (options?.get("sampleRate") as? Number)?.toInt() ?: 48000
      val channels = (options?.get("channels") as? Number)?.toInt() ?: 1
      val bitrate = (options?.get("bitrate") as? Number)?.toInt() ?: 24000
      val application = (options?.get("application") as? Number)?.toInt() ?: 1 // 1: Voip
      val enableMetering = (options?.get("enableMetering") as? Boolean) ?: true

      val channelConfig = if (channels == 2) {
        AudioFormat.CHANNEL_IN_STEREO
      } else {
        AudioFormat.CHANNEL_IN_MONO
      }

      val minBufferSize = AudioRecord.getMinBufferSize(
        sampleRate,
        channelConfig,
        AudioFormat.ENCODING_PCM_16BIT
      )
      val bufferSize = max(minBufferSize, sampleRate / 50 * channels * 2 * 4)

      val record = AudioRecord(
        MediaRecorder.AudioSource.MIC,
        sampleRate,
        channelConfig,
        AudioFormat.ENCODING_PCM_16BIT,
        bufferSize
      )

      if (record.state != AudioRecord.STATE_INITIALIZED) {
        record.release()
        throw Exception("Failed to initialize AudioRecord")
      }

      val fileName = "recording_${UUID.randomUUID()}.opus"
      val outputFile = File(context.cacheDir, fileName)
      recordingFilePath = outputFile.absolutePath

      val streamPtr = recorderCreate(
        outputFile.absolutePath,
        sampleRate,
        channels,
        bitrate,
        application
      )
      if (streamPtr == 0L) {
        record.release()
        throw Exception("Failed to initialize Opus native recorder")
      }

      recorderStreamPtr = streamPtr
      audioRecord = record
      isRecording = true
      isRecordingPaused = false

      record.startRecording()

      recordingThread = thread(start = true, name = "AudioOpusRecordingThread") {
        val chunkSamples = sampleRate / 50 * channels // 20ms frame
        val audioBuffer = ShortArray(chunkSamples)

        while (isRecording) {
          if (isRecordingPaused) {
            Thread.sleep(50)
            continue
          }

          val readSamples = record.read(audioBuffer, 0, audioBuffer.size)
          if (readSamples > 0) {
            val chunk = if (readSamples == audioBuffer.size) {
              audioBuffer
            } else {
              audioBuffer.copyOfRange(0, readSamples)
            }
            recorderWritePcm(streamPtr, chunk)

            if (enableMetering) {
              calculateMetering(chunk)
              sendEvent("onRecordingMetering", mapOf("amplitude" to currentAmplitude))
            }
          }
        }
      }
    }

    AsyncFunction("pauseRecording") {
      isRecordingPaused = true
    }

    AsyncFunction("resumeRecording") {
      isRecordingPaused = false
    }

    AsyncFunction("stopRecording") {
      if (!isRecording || recorderStreamPtr == 0L) {
        throw Exception("Not recording")
      }

      isRecording = false
      isRecordingPaused = false

      try {
        audioRecord?.stop()
      } catch (_: Exception) {}

      try {
        recordingThread?.join(1000)
      } catch (_: Exception) {}
      recordingThread = null

      audioRecord?.release()
      audioRecord = null

      val result = recorderFinish(recorderStreamPtr)
      recorderStreamPtr = 0L

      if (result == null || result.size < 2) {
        throw Exception("Failed to finalize Opus recording")
      }

      val durationMs = result[0]
      val fileSize = result[1]
      val path = recordingFilePath ?: ""

      mapOf(
        "uri" to "file://$path",
        "durationMs" to durationMs,
        "fileSize" to fileSize
      )
    }

    // -------------------------------------------------------------------
    // Playback (AudioTrack)
    // -------------------------------------------------------------------
    AsyncFunction("startPlayback") { uri: String ->
      if (isPlaying) {
        stopCurrentPlayback()
      }

      var cleanPath = uri
      if (cleanPath.startsWith("file://")) {
        cleanPath = cleanPath.substring(7)
      }

      val targetSampleRate = 48000
      val playerPtr = playerOpen(cleanPath, targetSampleRate)
      if (playerPtr == 0L) {
        throw Exception("Failed to open Opus audio file for playback")
      }

      playerStreamPtr = playerPtr
      val durationMs = playerGetDurationMs(playerPtr)
      val channels = playerGetChannels(playerPtr)
      playbackDurationMs = durationMs

      val channelConfig = if (channels == 2) {
        AudioFormat.CHANNEL_OUT_STEREO
      } else {
        AudioFormat.CHANNEL_OUT_MONO
      }

      val minBufferSize = AudioTrack.getMinBufferSize(
        targetSampleRate,
        channelConfig,
        AudioFormat.ENCODING_PCM_16BIT
      )

      val track = AudioTrack.Builder()
        .setAudioAttributes(
          AudioAttributes.Builder()
            .setUsage(AudioAttributes.USAGE_MEDIA)
            .setContentType(AudioAttributes.CONTENT_TYPE_SPEECH)
            .build()
        )
        .setAudioFormat(
          AudioFormat.Builder()
            .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
            .setSampleRate(targetSampleRate)
            .setChannelMask(channelConfig)
            .build()
        )
        .setBufferSizeInBytes(max(minBufferSize, 8192))
        .setTransferMode(AudioTrack.MODE_STREAM)
        .build()

      audioTrack = track
      isPlaying = true
      isPlaybackPaused = false

      track.play()

      playbackThread = thread(start = true, name = "AudioOpusPlaybackThread") {
        val pcmChunk = ShortArray(2048)
        while (isPlaying) {
          if (isPlaybackPaused) {
            Thread.sleep(50)
            continue
          }

          val read = playerReadPcm(playerPtr, pcmChunk)
          if (read <= 0) {
            // Reached end of audio
            sendEvent(
              "onPlaybackStatusUpdate",
              mapOf(
                "isPlaying" to false,
                "isPaused" to false,
                "positionMs" to durationMs,
                "durationMs" to durationMs,
                "didJustFinish" to true
              )
            )
            stopCurrentPlayback()
            break
          }

          track.write(pcmChunk, 0, read)

          val pos = playerGetPositionMs(playerPtr)
          sendEvent(
            "onPlaybackStatusUpdate",
            mapOf(
              "isPlaying" to (isPlaying && !isPlaybackPaused),
              "isPaused" to isPlaybackPaused,
              "positionMs" to pos,
              "durationMs" to durationMs,
              "didJustFinish" to false
            )
          )
        }
      }

      mapOf(
        "durationMs" to durationMs,
        "channels" to channels,
        "sampleRate" to targetSampleRate
      )
    }

    AsyncFunction("pausePlayback") {
      audioTrack?.pause()
      isPlaybackPaused = true
    }

    AsyncFunction("resumePlayback") {
      audioTrack?.play()
      isPlaybackPaused = false
    }

    AsyncFunction("stopPlayback") {
      stopCurrentPlayback()
    }

    AsyncFunction("seekTo") { positionMs: Double ->
      if (playerStreamPtr != 0L) {
        playerSeek(playerStreamPtr, max(0.0, positionMs).toLong())
      }
    }

    // -------------------------------------------------------------------
    // File & Buffer Conversions
    // -------------------------------------------------------------------
    AsyncFunction("encodeFile") { pcmPath: String, oggPath: String, options: Map<String, Any>? ->
      var cleanPcm = pcmPath
      if (cleanPcm.startsWith("file://")) cleanPcm = cleanPcm.substring(7)
      var cleanOgg = oggPath
      if (cleanOgg.startsWith("file://")) cleanOgg = cleanOgg.substring(7)

      val sampleRate = (options?.get("sampleRate") as? Number)?.toInt() ?: 48000
      val channels = (options?.get("channels") as? Number)?.toInt() ?: 1
      val bitrate = (options?.get("bitrate") as? Number)?.toInt() ?: 24000
      val application = (options?.get("application") as? Number)?.toInt() ?: 1

      val res = encodeFile(cleanPcm, cleanOgg, sampleRate, channels, bitrate, application)
        ?: throw Exception("encodeFile failed")

      mapOf(
        "uri" to "file://$cleanOgg",
        "durationMs" to res[0],
        "fileSize" to res[1]
      )
    }

    AsyncFunction("decodeFile") { oggPath: String, pcmPath: String, targetSampleRate: Int? ->
      var cleanOgg = oggPath
      if (cleanOgg.startsWith("file://")) cleanOgg = cleanOgg.substring(7)
      var cleanPcm = pcmPath
      if (cleanPcm.startsWith("file://")) cleanPcm = cleanPcm.substring(7)

      val rate = targetSampleRate ?: 48000
      val channels = decodeFile(cleanOgg, cleanPcm, rate)
      if (channels < 0) {
        throw Exception("decodeFile failed")
      }

      mapOf(
        "sampleRate" to rate,
        "channels" to channels
      )
    }

    AsyncFunction("encodePcmToOgg") { pcmBytes: ByteArray, options: Map<String, Any>? ->
      val sampleRate = (options?.get("sampleRate") as? Number)?.toInt() ?: 48000
      val channels = (options?.get("channels") as? Number)?.toInt() ?: 1
      val bitrate = (options?.get("bitrate") as? Number)?.toInt() ?: 24000
      val application = (options?.get("application") as? Number)?.toInt() ?: 1

      val shortCount = pcmBytes.size / 2
      val shorts = ShortArray(shortCount)
      for (i in 0 until shortCount) {
        val b1 = pcmBytes[i * 2].toInt() and 0xFF
        val b2 = pcmBytes[i * 2 + 1].toInt() shl 8
        shorts[i] = (b1 or b2).toShort()
      }

      encodeBuffer(shorts, sampleRate, channels, bitrate, application)
        ?: throw Exception("encodePcmToOgg failed")
    }

    AsyncFunction("decodeOggToPcm") { oggBytes: ByteArray, targetSampleRate: Int? ->
      val rate = targetSampleRate ?: 48000
      val pcmShorts = decodeBuffer(oggBytes, rate)
        ?: throw Exception("decodeOggToPcm failed")

      val byteData = ByteArray(pcmShorts.size * 2)
      for (i in pcmShorts.indices) {
        val s = pcmShorts[i].toInt()
        byteData[i * 2] = (s and 0xFF).toByte()
        byteData[i * 2 + 1] = ((s shr 8) and 0xFF).toByte()
      }

      mapOf(
        "pcm" to byteData,
        "sampleRate" to rate,
        "channels" to 1
      )
    }
  }

  private fun calculateMetering(pcm: ShortArray) {
    if (pcm.isEmpty()) return
    var sumSquares = 0.0
    for (sample in pcm) {
      val s = sample.toDouble() / 32768.0
      sumSquares += s * s
    }
    val rms = sqrt(sumSquares / pcm.size).toFloat()
    currentAmplitude = min(1.0f, max(0.0f, rms * 5.0f))
  }

  private fun stopCurrentPlayback() {
    isPlaying = false
    isPlaybackPaused = false

    if (Thread.currentThread() != playbackThread) {
      try {
        playbackThread?.join(500)
      } catch (_: Exception) {}
    }
    playbackThread = null

    audioTrack?.stop()
    audioTrack?.release()
    audioTrack = null

    if (playerStreamPtr != 0L) {
      playerDestroy(playerStreamPtr)
      playerStreamPtr = 0L
    }
  }

  companion object {
    init {
      System.loadLibrary("expo_audio_opus")
    }

    @JvmStatic external fun recorderCreate(filePath: String, sampleRate: Int, channels: Int, bitrate: Int, application: Int): Long
    @JvmStatic external fun recorderWritePcm(statePtr: Long, pcm: ShortArray): Int
    @JvmStatic external fun recorderFinish(statePtr: Long): LongArray?
    @JvmStatic external fun recorderDestroy(statePtr: Long)

    @JvmStatic external fun playerOpen(filePath: String, targetSampleRate: Int): Long
    @JvmStatic external fun playerReadPcm(statePtr: Long, outPcm: ShortArray): Int
    @JvmStatic external fun playerSeek(statePtr: Long, positionMs: Long): Int
    @JvmStatic external fun playerGetDurationMs(statePtr: Long): Long
    @JvmStatic external fun playerGetPositionMs(statePtr: Long): Long
    @JvmStatic external fun playerGetChannels(statePtr: Long): Int
    @JvmStatic external fun playerGetSampleRate(statePtr: Long): Int
    @JvmStatic external fun playerDestroy(statePtr: Long)

    @JvmStatic external fun encodeFile(pcmPath: String, oggPath: String, sampleRate: Int, channels: Int, bitrate: Int, application: Int): LongArray?
    @JvmStatic external fun decodeFile(oggPath: String, pcmPath: String, targetSampleRate: Int): Int
    @JvmStatic external fun encodeBuffer(pcm: ShortArray, sampleRate: Int, channels: Int, bitrate: Int, application: Int): ByteArray?
    @JvmStatic external fun decodeBuffer(ogg: ByteArray, targetSampleRate: Int): ShortArray?
  }
}
