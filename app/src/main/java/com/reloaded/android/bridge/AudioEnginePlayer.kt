package com.reloaded.android.bridge

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.util.Log
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Low-Latency PCM Float AudioTrack Player for Pokémon Reloaded.
 * Feeds 44.1kHz stereo samples pulled directly from reloaded-core via JNI.
 */
class AudioEnginePlayer {
    private var audioTrack: AudioTrack? = null
    private var playbackThread: Thread? = null
    private val isRunning = AtomicBoolean(false)
    private val isPaused = AtomicBoolean(false)

    companion object {
        private const val TAG = "AudioEnginePlayer"
        private const val SAMPLE_RATE = 44100
        private const val BUFFER_CHUNK_FLOATS = 1024
    }

    fun start() {
        if (isRunning.getAndSet(true)) return
        isPaused.set(false)

        try {
            val minBufferSize = AudioTrack.getMinBufferSize(
                SAMPLE_RATE,
                AudioFormat.CHANNEL_OUT_STEREO,
                AudioFormat.ENCODING_PCM_FLOAT
            ).coerceAtLeast(BUFFER_CHUNK_FLOATS * 4)

            val attributes = AudioAttributes.Builder()
                .setUsage(AudioAttributes.USAGE_GAME)
                .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                .build()

            val format = AudioFormat.Builder()
                .setSampleRate(SAMPLE_RATE)
                .setEncoding(AudioFormat.ENCODING_PCM_FLOAT)
                .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                .build()

            audioTrack = AudioTrack.Builder()
                .setAudioAttributes(attributes)
                .setAudioFormat(format)
                .setBufferSizeInBytes(minBufferSize)
                .setTransferMode(AudioTrack.MODE_STREAM)
                .setPerformanceMode(AudioTrack.PERFORMANCE_MODE_LOW_LATENCY)
                .build()

            audioTrack?.play()

            playbackThread = Thread({
                val buffer = FloatArray(BUFFER_CHUNK_FLOATS)
                while (isRunning.get()) {
                    if (isPaused.get()) {
                        try {
                            Thread.sleep(20)
                        } catch (_: InterruptedException) {
                            break
                        }
                        continue
                    }

                    val samplesRead = NativeBridge.pullAudioSamplesSafe(buffer)
                    if (samplesRead > 0) {
                        audioTrack?.write(buffer, 0, samplesRead, AudioTrack.WRITE_BLOCKING)
                    } else {
                        try {
                            Thread.sleep(5)
                        } catch (_: InterruptedException) {
                            break
                        }
                    }
                }
            }, "ReloadedAudioThread").apply {
                priority = Thread.MAX_PRIORITY
                start()
            }
            Log.i(TAG, "AudioEnginePlayer started with 44100Hz stereo float buffer")
        } catch (e: Throwable) {
            Log.e(TAG, "Failed to initialize AudioTrack: ${e.message}", e)
        }
    }

    fun pause() {
        isPaused.set(true)
        try {
            audioTrack?.pause()
        } catch (e: Throwable) {
            Log.w(TAG, "AudioTrack pause error: ${e.message}")
        }
    }

    fun resume() {
        isPaused.set(false)
        try {
            audioTrack?.play()
        } catch (e: Throwable) {
            Log.w(TAG, "AudioTrack resume error: ${e.message}")
        }
    }

    fun stop() {
        isRunning.set(false)
        isPaused.set(false)
        playbackThread?.interrupt()
        playbackThread = null

        try {
            audioTrack?.stop()
            audioTrack?.release()
        } catch (e: Throwable) {
            Log.w(TAG, "AudioTrack release error: ${e.message}")
        } finally {
            audioTrack = null
        }
        Log.i(TAG, "AudioEnginePlayer stopped")
    }
}
