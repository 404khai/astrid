package dev.astrid.overlay

import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.speech.RecognitionListener
import android.speech.RecognizerIntent
import android.speech.SpeechRecognizer
import android.speech.tts.TextToSpeech
import android.speech.tts.UtteranceProgressListener
import java.util.Locale

/**
 * The phone's voice mode (ADR 0039 amendment): Android's speech recogniser
 * for what you say (on-device when the phone has it, otherwise the system's,
 * asked to prefer offline) and its text-to-speech for Astrid's replies.
 * The mic is open only while listening; no audio is saved or sent by Astrid.
 */
class PhoneVoice(private val context: Context, private val emit: (String, Map<String, Any?>) -> Unit) {
  private val main = Handler(Looper.getMainLooper())
  private var recognizer: SpeechRecognizer? = null
  private var tts: TextToSpeech? = null
  private var ttsReady = false
  private var pendingSay: String? = null
  var onDevice = false
    private set
  /** The on-device recogniser failed on this phone: use the system's. */
  private var avoidOnDevice = false

  fun listen() = main.post { start() }

  private fun start() {
    recognizer?.destroy()
    recognizer = null
    if (!SpeechRecognizer.isRecognitionAvailable(context)) {
      emit("onVoice", mapOf("type" to "error", "message" to "Speech recognition isn't available on this phone."))
      return
    }
    onDevice = !avoidOnDevice && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S && SpeechRecognizer.isOnDeviceRecognitionAvailable(context)
    val r = if (onDevice) SpeechRecognizer.createOnDeviceSpeechRecognizer(context) else SpeechRecognizer.createSpeechRecognizer(context)
    r.setRecognitionListener(object : RecognitionListener {
      override fun onReadyForSpeech(params: Bundle?) = emit("onVoice", mapOf("type" to "listening", "onDevice" to onDevice))
      override fun onBeginningOfSpeech() {}
      override fun onRmsChanged(rmsdB: Float) = emit("onVoice", mapOf("type" to "level", "level" to ((rmsdB + 2f) / 12f).coerceIn(0f, 1f).toDouble()))
      override fun onBufferReceived(buffer: ByteArray?) {}
      override fun onEndOfSpeech() = emit("onVoice", mapOf("type" to "level", "level" to 0.0))
      override fun onError(error: Int) {
        android.util.Log.i("AstridVoice", "recogniser error $error (on-device: $onDevice)")
        // On-device recognition can be listed yet not installed: fall back once.
        if (onDevice && error != SpeechRecognizer.ERROR_NO_MATCH && error != SpeechRecognizer.ERROR_SPEECH_TIMEOUT &&
          error != SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS) {
          avoidOnDevice = true
          main.postDelayed({ start() }, 150)
          return
        }
        val quiet = error == SpeechRecognizer.ERROR_NO_MATCH || error == SpeechRecognizer.ERROR_SPEECH_TIMEOUT ||
          error == SpeechRecognizer.ERROR_CLIENT || error == SpeechRecognizer.ERROR_RECOGNIZER_BUSY
        emit("onVoice", mapOf("type" to if (quiet) "silence" else "error", "message" to describe(error), "code" to error))
      }
      override fun onResults(results: Bundle?) = emit("onVoice", mapOf("type" to "final", "text" to best(results)))
      override fun onPartialResults(results: Bundle?) = emit("onVoice", mapOf("type" to "partial", "text" to best(results)))
      override fun onEvent(eventType: Int, params: Bundle?) {}
    })
    val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
      .putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
      .putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true)
      .putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true)
      .putExtra(RecognizerIntent.EXTRA_SPEECH_INPUT_COMPLETE_SILENCE_LENGTH_MILLIS, 1300L)
      .putExtra(RecognizerIntent.EXTRA_LANGUAGE, Locale.getDefault().toLanguageTag())
    recognizer = r
    r.startListening(intent)
  }

  fun stopListening() {
    main.post {
      recognizer?.destroy()
      recognizer = null
    }
  }

  fun say(text: String) = main.post {
    val engine = tts ?: TextToSpeech(context) { status ->
      ttsReady = status == TextToSpeech.SUCCESS
      if (!ttsReady) emit("onVoice", mapOf("type" to "spoken"))
      pendingSay?.let { pendingSay = null; speakNow(it) }
    }.also { created ->
      tts = created
      created.setOnUtteranceProgressListener(object : UtteranceProgressListener() {
        override fun onStart(utteranceId: String?) = emit("onVoice", mapOf("type" to "speaking"))
        override fun onDone(utteranceId: String?) = emit("onVoice", mapOf("type" to "spoken"))
        @Deprecated("Deprecated in Java")
        override fun onError(utteranceId: String?) = emit("onVoice", mapOf("type" to "spoken"))
        override fun onStop(utteranceId: String?, interrupted: Boolean) = emit("onVoice", mapOf("type" to "spoken"))
      })
    }
    if (ttsReady) speakNow(text) else pendingSay = text
  }

  private fun speakNow(text: String) {
    tts?.speak(text, TextToSpeech.QUEUE_FLUSH, null, "astrid-${System.nanoTime()}")
  }

  fun hush() = main.post { tts?.stop() }

  fun shutdown() = main.post {
    recognizer?.destroy()
    recognizer = null
    tts?.shutdown()
    tts = null
    ttsReady = false
  }

  private fun best(results: Bundle?): String =
    results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)?.firstOrNull() ?: ""

  private fun describe(error: Int): String = when (error) {
    SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS -> "Astrid needs the microphone permission."
    SpeechRecognizer.ERROR_NETWORK, SpeechRecognizer.ERROR_NETWORK_TIMEOUT -> "Speech recognition needs a connection on this phone."
    SpeechRecognizer.ERROR_AUDIO -> "The microphone couldn't be opened."
    SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> "The recogniser is busy; try again."
    else -> "Speech recognition stopped (code $error). Tap Astrid to try again."
  }
}
