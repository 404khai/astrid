package dev.astrid.overlay

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.provider.Settings

/** Puts the bubble back after a restart, if it was on when the device went off. */
class PresenceBoot : BroadcastReceiver() {
  override fun onReceive(context: Context, intent: Intent) {
    if (intent.action != Intent.ACTION_BOOT_COMPLETED) return
    if (OverlayService.wanted(context) && Settings.canDrawOverlays(context)) OverlayService.start(context, false)
  }
}
