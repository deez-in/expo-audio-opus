import ExpoModulesCore
import AVFoundation

public class ExpoAudioOpusModule: Module {
    // Recorder state
    private var recorderStream: OpaquePointer?
    private var recordingEngine: AVAudioEngine?
    private var recordingFilePath: String?
    private var isRecording: Bool = false
    private var isRecordingPaused: Bool = false
    private var meteringTimer: Timer?
    private var currentAmplitude: Float = 0.0
    private var audioConverter: AVAudioConverter?
    private var targetFormat: AVAudioFormat?

    // Playback state
    private var playerDecoder: OpaquePointer?
    private var playerEngine: AVAudioEngine?
    private var playerNode: AVAudioPlayerNode?
    private var isPlaying: Bool = false
    private var isPlaybackPaused: Bool = false
    private var playbackTimer: Timer?
    private var playbackDurationMs: Int64 = 0

    public func definition() -> ModuleDefinition {
        Name("ExpoAudioOpus")

        Events("onRecordingMetering", "onPlaybackStatusUpdate")

        // -------------------------------------------------------------------
        // Permissions
        // -------------------------------------------------------------------
        AsyncFunction("requestPermissionsAsync") { () -> [String: Any] in
            var status = false
            let session = AVAudioSession.sharedInstance()
            let semaphore = DispatchSemaphore(value: 0)
            
            session.requestRecordPermission { granted in
                status = granted
                semaphore.signal()
            }
            semaphore.wait()

            return [
                "status": status ? "granted" : "denied",
                "granted": status,
                "canAskAgain": true
            ]
        }

        AsyncFunction("getPermissionsAsync") { () -> [String: Any] in
            let session = AVAudioSession.sharedInstance()
            let status: String
            let granted: Bool
            
            switch session.recordPermission {
            case .granted:
                status = "granted"
                granted = true
            case .denied:
                status = "denied"
                granted = false
            case .undetermined:
                status = "undetermined"
                granted = false
            @unknown default:
                status = "denied"
                granted = false
            }

            return [
                "status": status,
                "granted": granted,
                "canAskAgain": session.recordPermission == .undetermined
            ]
        }

        // -------------------------------------------------------------------
        // Recording (AVAudioEngine)
        // -------------------------------------------------------------------
        AsyncFunction("startRecording") { (options: [String: Any]?) in
            if self.isRecording {
                throw NSError(domain: "ExpoAudioOpus", code: 1, userInfo: [NSLocalizedDescriptionKey: "Already recording"])
            }

            let sampleRate = options?["sampleRate"] as? Int32 ?? 48000
            let channels = options?["channels"] as? Int32 ?? 1
            let bitrate = options?["bitrate"] as? Int32 ?? 24000
            let appMode = options?["application"] as? Int32 ?? 1 // 1: Voip
            let enableMetering = options?["enableMetering"] as? Bool ?? true

            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playAndRecord, mode: .voiceChat, options: [.defaultToSpeaker, .allowBluetooth])
            try session.setActive(true)

            let filename = "recording_\(UUID().uuidString).opus"
            let tempDir = NSTemporaryDirectory()
            let fullPath = (tempDir as NSString).appendingPathComponent(filename)
            self.recordingFilePath = fullPath

            guard let stream = opus_recorder_create(
                fullPath,
                sampleRate,
                channels,
                bitrate,
                appMode
            ) else {
                throw NSError(domain: "ExpoAudioOpus", code: 2, userInfo: [NSLocalizedDescriptionKey: "Failed to initialize Opus recorder"])
            }
            self.recorderStream = stream

            let engine = AVAudioEngine()
            self.recordingEngine = engine
            let inputNode = engine.inputNode
            let inputFormat = inputNode.inputFormat(forBus: 0)

            // Target format: 16-bit PCM at target sample rate and channel count
            guard let targetFmt = AVAudioFormat(
                commonFormat: .pcmFormatInt16,
                sampleRate: Double(sampleRate),
                channels: AVAudioChannelCount(channels),
                interleaved: true
            ) else {
                opus_recorder_destroy(stream)
                self.recorderStream = nil
                throw NSError(domain: "ExpoAudioOpus", code: 3, userInfo: [NSLocalizedDescriptionKey: "Failed to create target audio format"])
            }
            self.targetFormat = targetFmt

            if inputFormat != targetFmt {
                self.audioConverter = AVAudioConverter(from: inputFormat, to: targetFmt)
            } else {
                self.audioConverter = nil
            }

            self.isRecording = true
            self.isRecordingPaused = false

            inputNode.installTap(onBus: 0, bufferSize: 1024, format: inputFormat) { [weak self] (buffer, time) in
                guard let self = self, self.isRecording, !self.isRecordingPaused, let stream = self.recorderStream else {
                    return
                }

                if let converter = self.audioConverter, let targetFmt = self.targetFormat {
                    let frameCount = AVAudioFrameCount(Double(buffer.frameLength) * targetFmt.sampleRate / buffer.format.sampleRate)
                    guard let convertedBuffer = AVAudioPCMBuffer(pcmFormat: targetFmt, frameCapacity: frameCount) else {
                        return
                    }

                    var error: NSError? = nil
                    var haveData = true
                    let inputBlock: AVAudioConverterInputBlock = { inNumPackets, outStatus in
                        if haveData {
                            haveData = false
                            outStatus.pointee = .haveData
                            return buffer
                        } else {
                            outStatus.pointee = .noDataNow
                            return nil
                        }
                    }

                    converter.convert(to: convertedBuffer, error: &error, withInputFrom: inputBlock)
                    if error == nil, let pcmData = convertedBuffer.int16ChannelData {
                        let samples = pcmData.pointee
                        let count = Int(convertedBuffer.frameLength) * Int(targetFmt.channelCount)
                        opus_recorder_write_pcm(stream, samples, count)

                        if enableMetering {
                            self.calculateMetering(samples: samples, count: count)
                        }
                    }
                } else if let int16Data = buffer.int16ChannelData {
                    let samples = int16Data.pointee
                    let count = Int(buffer.frameLength) * Int(inputFormat.channelCount)
                    opus_recorder_write_pcm(stream, samples, count)

                    if enableMetering {
                        self.calculateMetering(samples: samples, count: count)
                    }
                } else if let floatData = buffer.floatChannelData {
                    // Convert float samples to int16
                    let count = Int(buffer.frameLength) * Int(inputFormat.channelCount)
                    var int16Samples = [Int16](repeating: 0, count: count)
                    let ptr = floatData.pointee
                    for i in 0..<count {
                        let sample = ptr[i]
                        let clamped = max(-1.0, min(1.0, sample))
                        int16Samples[i] = Int16(clamped * 32767.0)
                    }
                    opus_recorder_write_pcm(stream, &int16Samples, count)

                    if enableMetering {
                        int16Samples.withUnsafeBufferPointer { bufPtr in
                            if let base = bufPtr.baseAddress {
                                self.calculateMetering(samples: base, count: count)
                            }
                        }
                    }
                }
            }

            try engine.start()

            if enableMetering {
                DispatchQueue.main.async {
                    self.meteringTimer = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
                        guard let self = self, self.isRecording, !self.isRecordingPaused else { return }
                        self.sendEvent("onRecordingMetering", ["amplitude": self.currentAmplitude])
                    }
                }
            }
        }

        AsyncFunction("pauseRecording") {
            self.isRecordingPaused = true
        }

        AsyncFunction("resumeRecording") {
            self.isRecordingPaused = false
        }

        AsyncFunction("stopRecording") { () -> [String: Any] in
            guard self.isRecording, let engine = self.recordingEngine, let stream = self.recorderStream else {
                throw NSError(domain: "ExpoAudioOpus", code: 4, userInfo: [NSLocalizedDescriptionKey: "Not recording"])
            }

            self.meteringTimer?.invalidate()
            self.meteringTimer = nil

            engine.inputNode.removeTap(onBus: 0)
            engine.stop()
            self.recordingEngine = nil

            var durationMs: UInt64 = 0
            var fileSizeBytes: UInt64 = 0
            let res = opus_recorder_finish(stream, &durationMs, &fileSizeBytes)
            self.recorderStream = nil
            self.isRecording = false
            self.isRecordingPaused = false

            if res != 0 {
                throw NSError(domain: "ExpoAudioOpus", code: 5, userInfo: [NSLocalizedDescriptionKey: "Failed to finalize Opus recording"])
            }

            let path = self.recordingFilePath ?? ""
            return [
                "uri": "file://" + path,
                "durationMs": durationMs,
                "fileSize": fileSizeBytes
            ]
        }

        // -------------------------------------------------------------------
        // Playback (AVAudioEngine + AVAudioPlayerNode)
        // -------------------------------------------------------------------
        AsyncFunction("startPlayback") { (uri: String) -> [String: Any] in
            if self.isPlaying {
                self.stopCurrentPlayback()
            }

            var cleanPath = uri
            if cleanPath.hasPrefix("file://") {
                cleanPath = String(cleanPath.dropFirst(7))
            }

            let sampleRate: Int32 = 48000
            guard let player = opus_player_open(cleanPath, sampleRate) else {
                throw NSError(domain: "ExpoAudioOpus", code: 6, userInfo: [NSLocalizedDescriptionKey: "Failed to open Opus audio for playback"])
            }
            self.playerDecoder = player

            let duration = opus_player_get_duration_ms(player)
            let channels = opus_player_get_channels(player)
            self.playbackDurationMs = duration

            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.playback, mode: .default)
            try session.setActive(true)

            let engine = AVAudioEngine()
            let node = AVAudioPlayerNode()
            engine.attach(node)

            guard let format = AVAudioFormat(
                commonFormat: .pcmFormatInt16,
                sampleRate: Double(sampleRate),
                channels: AVAudioChannelCount(channels),
                interleaved: true
            ) else {
                opus_player_destroy(player)
                self.playerDecoder = nil
                throw NSError(domain: "ExpoAudioOpus", code: 7, userInfo: [NSLocalizedDescriptionKey: "Failed to create playback format"])
            }

            engine.connect(node, to: engine.mainMixerNode, format: format)
            try engine.start()
            // node.play() will be called by feedPlaybackBuffers after pre-buffering

            self.playerEngine = engine
            self.playerNode = node
            self.isPlaying = true
            self.isPlaybackPaused = false

            // Schedule audio chunks from decoder in background
            DispatchQueue.global(qos: .userInitiated).async { [weak self] in
                guard let self = self else { return }
                self.feedPlaybackBuffers(player: player, node: node, format: format)
            }

            DispatchQueue.main.async {
                self.playbackTimer = Timer.scheduledTimer(withTimeInterval: 0.1, repeats: true) { [weak self] _ in
                    guard let self = self, self.isPlaying, let p = self.playerDecoder else { return }
                    let pos = opus_player_get_position_ms(p)
                    self.sendEvent("onPlaybackStatusUpdate", [
                        "isPlaying": self.isPlaying && !self.isPlaybackPaused,
                        "isPaused": self.isPlaybackPaused,
                        "positionMs": pos,
                        "durationMs": self.playbackDurationMs,
                        "didJustFinish": false
                    ])
                }
            }

            return [
                "durationMs": duration,
                "channels": channels,
                "sampleRate": sampleRate
            ]
        }

        AsyncFunction("pausePlayback") {
            guard self.isPlaying, let node = self.playerNode else { return }
            node.pause()
            self.isPlaybackPaused = true
        }

        AsyncFunction("resumePlayback") {
            guard self.isPlaying, let node = self.playerNode else { return }
            node.play()
            self.isPlaybackPaused = false
        }

        AsyncFunction("stopPlayback") {
            self.stopCurrentPlayback()
        }

        AsyncFunction("seekTo") { (positionMs: Double) in
            guard let player = self.playerDecoder else { return }
            opus_player_seek(player, UInt64(max(0, positionMs)))
        }

        // -------------------------------------------------------------------
        // File & Buffer Conversions
        // -------------------------------------------------------------------
        AsyncFunction("encodeFile") { (pcmPath: String, oggPath: String, options: [String: Any]?) -> [String: Any] in
            var cleanPcm = pcmPath
            if cleanPcm.hasPrefix("file://") { cleanPcm = String(cleanPcm.dropFirst(7)) }
            var cleanOgg = oggPath
            if cleanOgg.hasPrefix("file://") { cleanOgg = String(cleanOgg.dropFirst(7)) }

            let sampleRate = options?["sampleRate"] as? Int32 ?? 48000
            let channels = options?["channels"] as? Int32 ?? 1
            let bitrate = options?["bitrate"] as? Int32 ?? 24000
            let appMode = options?["application"] as? Int32 ?? 1

            var durationMs: UInt64 = 0
            var fileSizeBytes: UInt64 = 0
            let res = opus_encode_file(cleanPcm, cleanOgg, sampleRate, channels, bitrate, appMode, &durationMs, &fileSizeBytes)
            if res != 0 {
                throw NSError(domain: "ExpoAudioOpus", code: 8, userInfo: [NSLocalizedDescriptionKey: "encodeFile failed with code \(res)"])
            }
            return [
                "uri": "file://" + cleanOgg,
                "durationMs": durationMs,
                "fileSize": fileSizeBytes
            ]
        }

        AsyncFunction("decodeFile") { (oggPath: String, pcmPath: String, targetSampleRate: Int32?) -> [String: Any] in
            var cleanOgg = oggPath
            if cleanOgg.hasPrefix("file://") { cleanOgg = String(cleanOgg.dropFirst(7)) }
            var cleanPcm = pcmPath
            if cleanPcm.hasPrefix("file://") { cleanPcm = String(cleanPcm.dropFirst(7)) }

            let rate = targetSampleRate ?? 48000
            var channels: Int32 = 0
            let res = opus_decode_file(cleanOgg, cleanPcm, rate, &channels)
            if res != 0 {
                throw NSError(domain: "ExpoAudioOpus", code: 9, userInfo: [NSLocalizedDescriptionKey: "decodeFile failed with code \(res)"])
            }
            return [
                "sampleRate": rate,
                "channels": channels
            ]
        }

        AsyncFunction("encodePcmToOgg") { (pcmData: Data, options: [String: Any]?) -> Data in
            let sampleRate = options?["sampleRate"] as? Int32 ?? 48000
            let channels = options?["channels"] as? Int32 ?? 1
            let bitrate = options?["bitrate"] as? Int32 ?? 24000
            let appMode = options?["application"] as? Int32 ?? 1

            let count = pcmData.count / 2
            var outPtr: UnsafeMutablePointer<UInt8>? = nil
            var outLen: Int = 0

            let result = pcmData.withUnsafeBytes { rawPtr -> Int32 in
                guard let pcmPtr = rawPtr.baseAddress?.assumingMemoryBound(to: Int16.self) else { return -1 }
                return opus_encode_buffer(pcmPtr, count, sampleRate, channels, bitrate, appMode, &outPtr, &outLen)
            }

            if result != 0 || outPtr == nil {
                throw NSError(domain: "ExpoAudioOpus", code: 10, userInfo: [NSLocalizedDescriptionKey: "encodePcmToOgg failed"])
            }

            defer {
                if let ptr = outPtr {
                    opus_free_buffer(ptr, outLen)
                }
            }

            return Data(bytes: outPtr!, count: outLen)
        }

        AsyncFunction("decodeOggToPcm") { (oggData: Data, targetSampleRate: Int32?) -> [String: Any] in
            let rate = targetSampleRate ?? 48000
            var outPcm: UnsafeMutablePointer<Int16>? = nil
            var outCount: Int = 0
            var outChannels: Int32 = 0

            let result = oggData.withUnsafeBytes { rawPtr -> Int32 in
                guard let oggPtr = rawPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else { return -1 }
                return opus_decode_buffer(oggPtr, oggData.count, rate, &outPcm, &outCount, &outChannels)
            }

            if result != 0 || outPcm == nil {
                throw NSError(domain: "ExpoAudioOpus", code: 11, userInfo: [NSLocalizedDescriptionKey: "decodeOggToPcm failed"])
            }

            defer {
                if let ptr = outPcm {
                    opus_free_pcm_buffer(ptr, outCount)
                }
            }

            let byteCount = outCount * 2
            let pcmBytes = Data(bytes: outPcm!, count: byteCount)
            return [
                "pcm": pcmBytes,
                "sampleRate": rate,
                "channels": outChannels
            ]
        }
    }

    private func calculateMetering(samples: UnsafePointer<Int16>, count: Int) {
        if count == 0 { return }
        var sumSquares: Double = 0.0
        for i in 0..<count {
            let s = Double(samples[i]) / 32768.0
            sumSquares += s * s
        }
        let rms = Float(sqrt(sumSquares / Double(count)))
        // Normalize roughly between 0.0 and 1.0
        self.currentAmplitude = min(1.0, max(0.0, rms * 5.0))
    }

    private func feedPlaybackBuffers(player: OpaquePointer, node: AVAudioPlayerNode, format: AVAudioFormat) {
        let chunkSize = 4096
        var pcmBuffer = [Int16](repeating: 0, count: chunkSize)
        let maxBuffers = 4
        let semaphore = DispatchSemaphore(value: maxBuffers)
        var buffersScheduled = 0
        var hasStartedPlaying = false

        while self.isPlaying {
            if self.isPlaybackPaused {
                usleep(50_000)
                continue
            }

            semaphore.wait()

            if !self.isPlaying {
                semaphore.signal()
                break
            }

            let read = opus_player_read_pcm(player, &pcmBuffer, chunkSize)
            if read <= 0 {
                // End of stream
                if !hasStartedPlaying {
                    node.play()
                    hasStartedPlaying = true
                }
                
                // Wait for all in-flight buffers to finish playing
                for _ in 0..<maxBuffers {
                    semaphore.wait()
                }
                
                if self.isPlaying {
                    DispatchQueue.main.async {
                        self.sendEvent("onPlaybackStatusUpdate", [
                            "isPlaying": false,
                            "isPaused": false,
                            "positionMs": self.playbackDurationMs,
                            "durationMs": self.playbackDurationMs,
                            "didJustFinish": true
                        ])
                        self.stopCurrentPlayback()
                    }
                }
                
                for _ in 0..<maxBuffers {
                    semaphore.signal()
                }
                break
            }

            let frameCount = AVAudioFrameCount(read / Int32(format.channelCount))
            guard let audioBuffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frameCount) else {
                semaphore.signal()
                break
            }
            audioBuffer.frameLength = frameCount

            if let targetPtr = audioBuffer.int16ChannelData?.pointee {
                for i in 0..<Int(read) {
                    targetPtr[i] = pcmBuffer[i]
                }
            }

            node.scheduleBuffer(audioBuffer) {
                semaphore.signal()
            }
            buffersScheduled += 1
            
            if !hasStartedPlaying && buffersScheduled >= 2 {
                node.play()
                hasStartedPlaying = true
            }
        }
    }

    private func stopCurrentPlayback() {
        self.playbackTimer?.invalidate()
        self.playbackTimer = nil

        self.playerNode?.stop()
        self.playerEngine?.stop()
        self.playerEngine = nil
        self.playerNode = nil

        if let player = self.playerDecoder {
            opus_player_destroy(player)
            self.playerDecoder = nil
        }
        self.isPlaying = false
        self.isPlaybackPaused = false
    }
}
