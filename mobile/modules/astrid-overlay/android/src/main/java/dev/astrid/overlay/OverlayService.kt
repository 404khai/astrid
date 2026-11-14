package dev.astrid.overlay

import android.annotation.SuppressLint
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.graphics.PixelFormat
import android.os.Build
import android.os.IBinder
import android.provider.Settings
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.WindowManager
import kotlin.math.abs
import kotlin.math.hypot

/**
 * Astrid on top of other apps (ADR 0039): a small bubble with her mark that you
 * drag anywhere and tap to open the ask panel over whatever is on screen. A
 * foreground service with a notification keeps it there; the notification's
 * Stop, or the app's switch, takes it away. The bubble shows only Astrid's own
 * mark: it reads nothing from the screen and sends nothing anywhere.
 */
class OverlayService : Service() {
  private var bubble: View? = null
  private lateinit var params: WindowManager.LayoutParams

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    if (intent?.action == ACTION_STOP) {
      stopSelf()
      return START_NOT_STICKY
    }
    if (!Settings.canDrawOverlays(this)) {
      stopSelf()
      return START_NOT_STICKY
    }
    startForegroundNotice()
    if (bubble == null) {
      showBubble()
      running = true
      // The first time it comes on: the wave round the screen's edges.
      if (intent?.getBooleanExtra(EXTRA_WAVE, true) != false) EdgeWave.play(this)
    }
    return START_STICKY
  }

  override fun onDestroy() {
    bubble?.let { runCatching { getSystemService(WindowManager::class.java).removeView(it) } }
    bubble = null
    running = false
    super.onDestroy()
  }

  private fun startForegroundNotice() {
    val manager = getSystemService(NotificationManager::class.java)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      manager.createNotificationChannel(NotificationChannel(CHANNEL, "Astrid on screen", NotificationManager.IMPORTANCE_MIN).apply {
        description = "Shown while Astrid's bubble is on top of other apps."
        setShowBadge(false)
      })
    }
    val stop = PendingIntent.getService(
      this, 1, Intent(this, OverlayService::class.java).setAction(ACTION_STOP),
      PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
    val open = PendingIntent.getActivity(this, 2, AskActivity.intent(this), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
    val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) Notification.Builder(this, CHANNEL) else @Suppress("DEPRECATION") Notification.Builder(this)
    val notice = builder
      .setSmallIcon(R.drawable.astrid_notification)
      .setContentTitle("Astrid is on screen")
      .setContentText("Tap the bubble to ask. Stop to take it away.")
      .setContentIntent(open)
      .setOngoing(true)
      .addAction(Notification.Action.Builder(null, "Stop", stop).build())
      .build()
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
      startForeground(NOTICE, notice, ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE)
    } else {
      startForeground(NOTICE, notice)
    }
  }

  @SuppressLint("ClickableViewAccessibility")
  private fun showBubble() {
    val manager = getSystemService(WindowManager::class.java)
    val density = resources.displayMetrics.density
    val size = (72 * density).toInt()
    val view = BubbleView(this).apply { contentDescription = "Ask Astrid" }
    params = WindowManager.LayoutParams(
      size, size,
      WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
      WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
      PixelFormat.TRANSLUCENT,
    ).apply {
      gravity = Gravity.TOP or Gravity.START
      x = resources.displayMetrics.widthPixels - size - (12 * density).toInt()
      y = (resources.displayMetrics.heightPixels * 0.62).toInt()
      title = "Astrid bubble"
    }
    val slop = ViewConfiguration.get(this).scaledTouchSlop
    var downX = 0f
    var downY = 0f
    var startX = 0
    var startY = 0
    var dragging = false
    view.setOnTouchListener { _, event ->
      when (event.actionMasked) {
        MotionEvent.ACTION_DOWN -> {
          downX = event.rawX; downY = event.rawY; startX = params.x; startY = params.y; dragging = false
          view.animate().scaleX(0.92f).scaleY(0.92f).setDuration(90).start()
        }
        MotionEvent.ACTION_MOVE -> {
          val dx = event.rawX - downX
          val dy = event.rawY - downY
          if (!dragging && hypot(dx, dy) > slop) dragging = true
          if (dragging) {
            params.x = startX + dx.toInt()
            params.y = startY + dy.toInt()
            manager.updateViewLayout(view, params)
          }
        }
        MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
          view.animate().scaleX(1f).scaleY(1f).setDuration(120).start()
          if (dragging) {
            snap(manager, view, size, density)
          } else if (event.actionMasked == MotionEvent.ACTION_UP) {
            view.performHapticFeedback(android.view.HapticFeedbackConstants.CONTEXT_CLICK)
            openPanel()
          }
        }
      }
      true
    }
    manager.addView(view, params)
    bubble = view
  }

  /** The ask panel over the current app; the app itself if that fails. */
  private fun openPanel() {
    try {
      startActivity(AskActivity.intent(this))
    } catch (error: Throwable) {
      android.util.Log.e("AstridOverlay", "ask panel failed to open", error)
      packageManager.getLaunchIntentForPackage(packageName)?.let { startActivity(it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
    }
  }

  /** Back to the nearer side after a drag, kept clear of the top and bottom. */
  private fun snap(manager: WindowManager, view: View, size: Int, density: Float) {
    val metrics = resources.displayMetrics
    val margin = (12 * density).toInt()
    val left = params.x + size / 2 < metrics.widthPixels / 2
    val targetX = if (left) margin else metrics.widthPixels - size - margin
    val targetY = params.y.coerceIn((48 * density).toInt(), metrics.heightPixels - size - (72 * density).toInt())
    val fromX = params.x
    val fromY = params.y
    android.animation.ValueAnimator.ofFloat(0f, 1f).apply {
      duration = if (abs(targetX - fromX) > 0) 220 else 0
      interpolator = android.view.animation.DecelerateInterpolator()
      addUpdateListener {
        val t = it.animatedValue as Float
        params.x = (fromX + (targetX - fromX) * t).toInt()
        params.y = (fromY + (targetY - fromY) * t).toInt()
        runCatching { manager.updateViewLayout(view, params) }
      }
      start()
    }
  }

  companion object {
    private const val CHANNEL = "astrid-overlay"
    private const val NOTICE = 47821
    const val ACTION_STOP = "dev.astrid.overlay.STOP"
    const val EXTRA_WAVE = "wave"
    @Volatile var running = false

    fun start(context: Context, wave: Boolean) {
      val intent = Intent(context, OverlayService::class.java).putExtra(EXTRA_WAVE, wave)
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) context.startForegroundService(intent) else context.startService(intent)
    }

    fun stop(context: Context) {
      context.stopService(Intent(context, OverlayService::class.java))
    }
  }
}
