package dev.astrid.overlay

import android.app.Activity
import android.os.Bundle
import android.provider.Settings

/**
 * The Android TV home-screen tile: puts Astrid's bubble up and opens the ask
 * panel, without the full app. Without the overlay permission it opens the app,
 * which explains how to allow it.
 */
class PresenceLaunch : Activity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    if (Settings.canDrawOverlays(this)) {
      if (!OverlayService.running) OverlayService.start(this, true)
      startActivity(AskActivity.intent(this))
    } else {
      packageManager.getLaunchIntentForPackage(packageName)?.let { startActivity(it) }
    }
    finish()
  }
}
