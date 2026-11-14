package dev.astrid.overlay

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Color
import android.view.MotionEvent
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout

/**
 * The floating bubble: Astrid's mark in presence's voice ring, the same page
 * (`web/voice.html`, built into the APK) as the voice screen and the desktop,
 * drawn at the bubble's size on a transparent background. The bubble keeps
 * every touch for itself, so dragging and tapping work as before.
 */
@SuppressLint("SetJavaScriptEnabled", "ViewConstructor")
class BubbleView(context: Context, sizeDp: Int) : FrameLayout(context) {
  private val web = WebView(context).apply {
    setBackgroundColor(Color.TRANSPARENT)
    isVerticalScrollBarEnabled = false
    isHorizontalScrollBarEnabled = false
    overScrollMode = OVER_SCROLL_NEVER
    settings.javaScriptEnabled = true
    settings.allowFileAccess = true
    settings.setSupportMultipleWindows(false)
    // Only the ring page; nothing else loads.
    webViewClient = object : WebViewClient() {
      override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest) = true
    }
    loadUrl("file:///android_asset/astrid-voice.html?size=$sizeDp")
  }

  init {
    addView(web, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT))
  }

  override fun onInterceptTouchEvent(ev: MotionEvent): Boolean = true

  override fun onDetachedFromWindow() {
    web.destroy()
    super.onDetachedFromWindow()
  }
}
