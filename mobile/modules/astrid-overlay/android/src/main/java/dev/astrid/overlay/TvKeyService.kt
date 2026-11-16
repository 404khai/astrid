package dev.astrid.overlay

import android.accessibilityservice.AccessibilityService
import android.provider.Settings
import android.view.KeyEvent
import android.view.accessibility.AccessibilityEvent

/**
 * Astrid on a TV (ADR 0039 amendment): the remote has no way to reach a
 * floating bubble, so one remote button opens the ask panel instead. Android
 * passes every key through a key-filtering accessibility service; this one
 * looks only at [KEY] and lets everything else through untouched. It asks for
 * no window content or accessibility events, and keeps and sends nothing.
 */
class TvKeyService : AccessibilityService() {
  override fun onKeyEvent(event: KeyEvent): Boolean {
    if (event.keyCode != KEY || !Settings.canDrawOverlays(this)) return false
    if (event.action == KeyEvent.ACTION_UP && !event.isCanceled) {
      if (!OverlayService.running) OverlayService.start(this, false)
      startActivity(AskActivity.intent(this))
    }
    return true
  }

  override fun onAccessibilityEvent(event: AccessibilityEvent?) = Unit
  override fun onInterrupt() = Unit

  companion object {
    /** The remote's TV button. */
    @Volatile var KEY = KeyEvent.KEYCODE_TV
  }
}
