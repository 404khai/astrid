package dev.astrid.overlay

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.SweepGradient
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.ImageView
import kotlin.math.sin

/**
 * The floating bubble: Astrid's mark with no disc behind it, inside her
 * voice ring, rings in her palette that slowly turn and breathe, as the
 * presence and desktop rings do.
 */
class BubbleView(context: Context) : FrameLayout(context) {
  private val density = resources.displayMetrics.density
  private val colors = intArrayOf(
    Color.rgb(68, 89, 249), Color.rgb(0, 247, 213), Color.rgb(138, 92, 255), Color.rgb(229, 72, 127), Color.rgb(68, 89, 249),
  )
  private val ring = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
  private val turn = Matrix()
  private var time = 0f
  private val clock = ValueAnimator.ofFloat(0f, 1f).apply {
    duration = 1000
    repeatCount = ValueAnimator.INFINITE
    interpolator = LinearInterpolator()
    addUpdateListener {
      time += 1f / 60f
      invalidate()
    }
  }

  init {
    setWillNotDraw(false)
    val mark = ImageView(context).apply { setImageResource(R.drawable.astrid_mark) }
    val inset = (16 * density).toInt()
    addView(mark, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.MATCH_PARENT).apply { setMargins(inset, inset, inset, inset) })
  }

  override fun onAttachedToWindow() {
    super.onAttachedToWindow()
    clock.start()
  }

  override fun onDetachedFromWindow() {
    clock.cancel()
    super.onDetachedFromWindow()
  }

  override fun onDraw(canvas: Canvas) {
    val cx = width / 2f
    val cy = height / 2f
    val gradient = SweepGradient(cx, cy, colors, null)
    turn.setRotate(time * 40f, cx, cy)
    gradient.setLocalMatrix(turn)
    ring.shader = gradient
    val base = minOf(cx, cy) - 4 * density
    val breath = 0.5f + 0.5f * sin(time * 2.2f)
    for (i in 0 until 3) {
      ring.strokeWidth = (2.2f - i * 0.6f) * density
      ring.alpha = ((0.95f - i * 0.28f) * 255).toInt()
      val r = base * (0.86f + i * 0.07f + breath * 0.05f * (i + 1))
      canvas.drawCircle(cx, cy, minOf(r, base), ring)
    }
    super.onDraw(canvas)
  }
}
