package dev.astrid.overlay

import android.animation.ValueAnimator
import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.PixelFormat
import android.graphics.RectF
import android.graphics.RuntimeShader
import android.graphics.SweepGradient
import android.os.Build
import android.provider.Settings
import android.util.Log
import android.view.RoundedCorner
import android.view.View
import android.view.WindowManager
import android.view.animation.LinearInterpolator

/**
 * The intro wave on the phone (ADR 0039): when the overlay is switched on, the
 * Distortion Waves band rises from the bottom middle of the screen to the top,
 * leaves its colour on the screen's edges and goes, as presence's does on the
 * Mac. Drawn in its own window that takes no touches, focus or screen content;
 * it removes itself when the wave has cleared. Android 13+ draws the band with
 * an AGSL shader; older phones get a plain glow round the edges.
 */
@SuppressLint("ViewConstructor")
class EdgeWave private constructor(context: Context, private val cornerDp: Double) : View(context) {
  private val density = resources.displayMetrics.density
  private val shader: RuntimeShader? = makeShader()
  private val band = Paint().apply { shader = this@EdgeWave.shader }
  private val spark = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.WHITE }
  private val glow = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
  private val particles = (0 until WaveStyle.particleCount).map { WaveParticle(it) }
  private var progress = 0.0

  private fun makeShader(): RuntimeShader? {
    if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return null
    return try {
      RuntimeShader(WAVE_BAND_AGSL).apply {
        setFloatUniform("palette", WaveStyle.bandPalette)
      }
    } catch (error: Throwable) {
      Log.w("AstridOverlay", "wave shader unavailable: $error")
      null
    }
  }

  private fun source(width: Double, height: Double): Box {
    // A pill at the bottom middle, as from a button there.
    val w = width * 0.62
    return Box(width / 2 - w / 2, height - 76, w, 44.0)
  }

  override fun onDraw(canvas: Canvas) {
    val width = this.width / density.toDouble()
    val height = this.height / density.toDouble()
    if (width <= 0 || height <= 0) return
    val source = source(width, height)
    val front = WaveFront(progress, source, width, height)
    val shader = shader
    if (shader != null && Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
      shader.setFloatUniform("scale", density)
      shader.setFloatUniform("size", width.toFloat(), height.toFloat())
      shader.setFloatUniform("origin", front.originX.toFloat(), front.originY.toFloat())
      shader.setFloatUniform("spine", front.spine.toFloat())
      shader.setFloatUniform("radius", front.radius.toFloat())
      shader.setFloatUniform("squash", front.squash.toFloat())
      shader.setFloatUniform("time", front.time.toFloat())
      shader.setFloatUniform("swell", front.swell.toFloat())
      shader.setFloatUniform("glow", front.glow.toFloat())
      shader.setFloatUniform("clearRadius", front.clearRadius.toFloat())
      shader.setFloatUniform("rim", front.rim.toFloat())
      shader.setFloatUniform("hairClear", front.hairClearRadius.toFloat())
      shader.setFloatUniform("cornerRadius", cornerDp.toFloat())
      shader.setFloatUniform("lumps", WaveRim.lumps(front.time, width, height, cornerDp, WaveStyle.rimPulse))
      band.shader = shader
      canvas.drawPaint(band)
    } else {
      drawGlow(canvas, front)
    }
    if (front.time > 0) {
      for (particle in particles) {
        val opacity = particle.opacity(front)
        if (opacity <= 0) continue
        val (x, y) = particle.point(front, source)
        spark.alpha = (opacity.coerceIn(0.0, 1.0) * 255).toInt()
        canvas.drawCircle((x * density).toFloat(), (y * density).toFloat(), (particle.size / 2 * density).toFloat(), spark)
      }
    }
  }

  /** Before Android 13: the palette as a soft line round the screen's edges. */
  private fun drawGlow(canvas: Canvas, front: WaveFront) {
    val strength = (front.rim / WaveStyle.rimStrength).coerceIn(0.0, 1.0) *
      (1 - WaveFront.smooth(0.55, 1.0, progress))
    if (strength <= 0) return
    val colors = (WaveStyle.rgb + listOf(WaveStyle.rgb.first())).map { Color.rgb(it[0], it[1], it[2]) }.toIntArray()
    glow.shader = SweepGradient(width / 2f, height / 2f, colors, null).apply {
      setLocalMatrix(android.graphics.Matrix().apply { setRotate((front.time * 60).toFloat(), width / 2f, height / 2f) })
    }
    val radius = (cornerDp * density).toFloat()
    for ((stroke, alpha) in listOf(18f to 0.25, 8f to 0.45, 2f to 0.9)) {
      glow.strokeWidth = stroke * density
      glow.alpha = (alpha * strength * 255).toInt()
      val inset = glow.strokeWidth / 2
      canvas.drawRoundRect(RectF(inset, inset, width - inset, height - inset), radius, radius, glow)
    }
  }

  companion object {
    private var playing: EdgeWave? = null

    /** Plays the wave once over everything. Does nothing under reduced motion. */
    fun play(context: Context) {
      if (playing != null || reducedMotion(context)) return
      val manager = context.getSystemService(WindowManager::class.java) ?: return
      val density = context.resources.displayMetrics.density
      var corner = 24.0
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        val rounded = manager.currentWindowMetrics.windowInsets.getRoundedCorner(RoundedCorner.POSITION_TOP_LEFT)
        if (rounded != null && rounded.radius > 0) corner = rounded.radius / density.toDouble()
      }
      val view = EdgeWave(context, corner)
      val params = WindowManager.LayoutParams(
        WindowManager.LayoutParams.MATCH_PARENT,
        WindowManager.LayoutParams.MATCH_PARENT,
        WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
        WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE or WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
          WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS or
          WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED,
        PixelFormat.TRANSLUCENT,
      ).apply {
        title = "Astrid intro wave"
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
          layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
          layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS
          fitInsetsTypes = 0
        }
        // Touches pass through an overlay of another app only up to this
        // opacity (Android 12+), so the phone stays usable while it plays.
        alpha = 0.8f
      }
      try {
        manager.addView(view, params)
      } catch (error: Throwable) {
        Log.w("AstridOverlay", "wave window refused: $error")
        return
      }
      playing = view
      ValueAnimator.ofFloat(0f, 1f).apply {
        duration = (WaveStyle.duration * 1000).toLong()
        interpolator = LinearInterpolator()
        addUpdateListener {
          view.progress = (it.animatedValue as Float).toDouble()
          view.invalidate()
        }
        doOnEnd {
          runCatching { manager.removeView(view) }
          playing = null
        }
        start()
      }
    }

    private fun reducedMotion(context: Context): Boolean {
      val scale = Settings.Global.getFloat(context.contentResolver, Settings.Global.ANIMATOR_DURATION_SCALE, 1f)
      return scale == 0f
    }

    private fun ValueAnimator.doOnEnd(block: () -> Unit) {
      addListener(object : android.animation.AnimatorListenerAdapter() {
        override fun onAnimationEnd(animation: android.animation.Animator) = block()
      })
    }
  }
}
