package dev.astrid.overlay

import android.content.Intent
import android.net.Uri
import android.provider.Settings
import expo.modules.kotlin.modules.Module
import expo.modules.kotlin.modules.ModuleDefinition

/** The overlay's controls for the app's JavaScript (ADR 0039). */
class AstridOverlayModule : Module() {
  private val context get() = requireNotNull(appContext.reactContext)

  private var voice: PhoneVoice? = null
  private fun voice(): PhoneVoice = voice ?: PhoneVoice(context) { name, body -> sendEvent(name, body) }.also { voice = it }

  override fun definition() = ModuleDefinition {
    Name("AstridOverlay")

    Events("onVoice")

    /** Voice mode: start listening (events come as onVoice). */
    Function("listen") { voice().listen() }
    Function("stopListening") { voice?.stopListening() }
    Function("say") { text: String -> voice().say(text.take(4000)) }
    Function("hush") { voice?.hush() }
    OnDestroy { voice?.shutdown() }

    Function("canDrawOverlays") { Settings.canDrawOverlays(context) }

    /** Opens the system page where the user allows drawing over other apps. */
    Function("openOverlaySettings") {
      val intent = Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:${context.packageName}"))
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      context.startActivity(intent)
    }

    Function("isRunning") { OverlayService.running }

    /** Shows the bubble; with `wave`, the intro wave plays round the edges first. */
    Function("start") { wave: Boolean ->
      if (!Settings.canDrawOverlays(context)) return@Function false
      OverlayService.start(context, wave)
      true
    }

    Function("stop") { OverlayService.stop(context) }

    /** Plays the intro wave alone, for the app's own preview. */
    Function("playWave") {
      if (Settings.canDrawOverlays(context)) EdgeWave.play(context)
    }

    /** Closes the ask panel, if this is it. */
    Function("closePanel") {
      (appContext.currentActivity as? AskActivity)?.finish()
    }
  }
}
