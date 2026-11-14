// Ported from Libraries.dev Distortion Waves (DistortionWavesKit),
// https://github.com/Jakubantalik/Libraries.dev, commit 387b049, by way of
// presence's vendored copy (apps/presence-mac/Sources/AstridPresence/Waves).
// MIT License, Copyright (c) 2026 Jakub Antalik: see
// apps/presence-mac/ThirdParty/distortion-waves/LICENSE.
// Only the vertical wave, untouched by a finger, as the phone intro plays it
// (ADR 0039). Lengths are in dp, as the original's are in points.
package dev.astrid.overlay

import kotlin.math.PI
import kotlin.math.cbrt
import kotlin.math.floor
import kotlin.math.hypot
import kotlin.math.max
import kotlin.math.min
import kotlin.math.pow
import kotlin.math.sin
import kotlin.math.sqrt

/** DistortionWaveStyle.vertical: the numbers that shape the wave. */
object WaveStyle {
  const val anticipation = 0.5
  const val travel = 2.28
  const val linger = -0.35
  const val rimFade = 3.03
  val rimEasing = Easing(0.42, 0.0, 0.58, 1.0)
  val easing = Easing(0.22, 1.0, 0.36, 1.0)
  const val inhale = 56.0
  const val gatherGlow = 0.33
  const val gatherDots = 1.0
  const val swell = 22.0
  const val lead = 130.0
  const val tail = 300.0
  const val strength = 0.92
  const val rimWidth = 14.0
  const val rimUnevenness = 1.0
  const val rimStrength = 0.82
  const val rimSoftness = 343.0
  const val rimPulse = 0.6
  const val hairlineWidth = 1.5
  const val hairlineOpacity = 0.9
  const val hairlineWhite = 0.95
  const val hairlineLinger = 0.47
  const val hairlineFade = 3.0
  const val particleCount = 64
  const val particleSize = 1.8
  const val particleReach = 150.0
  const val particleFlight = 30.0
  const val particleGap = 24.0
  const val particleLife = 2.19
  const val particleSpread = 1.0
  val duration = anticipation + maxOf(travel * 0.6, linger + rimFade, linger + hairlineLinger + hairlineFade, particleLife)

  /** The colourful palette, then the warm and cool fringes. */
  private val colors = listOf(
    intArrayOf(255, 201, 21), intArrayOf(126, 196, 255), intArrayOf(180, 40, 230), intArrayOf(235, 100, 160),
    intArrayOf(255, 176, 122), intArrayOf(154, 160, 255), intArrayOf(127, 217, 238),
    intArrayOf(255, 122, 182), intArrayOf(90, 190, 255),
  )
  const val paletteColors = 7

  /** The palette as OKLab triplets, flattened, as the shader reads it. */
  val bandPalette: FloatArray = colors.flatMap { oklab(it).toList() }.toFloatArray()

  /** sRGB for the fallback rim, which can't mix in OKLab. */
  val rgb: List<IntArray> = colors.take(paletteColors)

  private fun oklab(c: IntArray): FloatArray {
    fun linear(v: Int): Double {
      val x = v / 255.0
      return if (x <= 0.04045) x / 12.92 else ((x + 0.055) / 1.055).pow(2.4)
    }
    val (lr, lg, lb) = Triple(linear(c[0]), linear(c[1]), linear(c[2]))
    val l = cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb)
    val m = cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb)
    val s = cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb)
    return floatArrayOf(
      (0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s).toFloat(),
      (1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s).toFloat(),
      (0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s).toFloat(),
    )
  }
}

/** A cubic Bézier easing, as CSS's cubic-bezier(). */
class Easing(private val x1: Double, private val y1: Double, private val x2: Double, private val y2: Double) {
  operator fun invoke(x: Double): Double {
    if (x <= 0) return 0.0
    if (x >= 1) return 1.0
    var low = 0.0
    var high = 1.0
    repeat(24) {
      val s = (low + high) / 2
      if (bezier(s, x1, x2) < x) low = s else high = s
    }
    return bezier((low + high) / 2, y1, y2)
  }

  private fun bezier(s: Double, a: Double, b: Double): Double {
    val r = 1 - s
    return 3 * r * r * s * a + 3 * r * s * s * b + s * s * s
  }
}

/** A rectangle in dp, from the top left. */
data class Box(val x: Double, val y: Double, val width: Double, val height: Double) {
  val midX get() = x + width / 2
  val midY get() = y + height / 2
}

/** WaveFront: where the wave is at a moment of its life, a pure function of progress. */
class WaveFront(progress: Double, source: Box, val width: Double, val height: Double) {
  val originX: Double
  val originY: Double
  val spine: Double
  val radius: Double
  val moved: Double
  val squash: Double
  val time: Double
  val swell: Double
  val glow: Double
  val clearRadius: Double
  val hairClearRadius: Double
  val rim: Double
  val sparkle: Double
  val flying: Double
  val launched: Boolean

  init {
    val s = WaveStyle
    val t = min(max(progress, 0.0), 1.0) * s.duration
    val sinceLaunch = t - s.anticipation
    val gathered = smooth(0.0, max(s.anticipation, 0.001), t)
    val x = min(max(sinceLaunch / s.travel, 0.0), 1.0)
    moved = s.easing(x)
    val fullSquash = ARC_SQUASH
    val startRadius = min(source.width, source.height) / 2
    val startSpine = max(0.0, source.width / 2 - startRadius)
    val opening = smooth(0.0, 0.45, moved)
    var reach = 0.0
    for ((cx, cy) in listOf(0.0 to 0.0, width to 0.0, 0.0 to height, width to height)) {
      reach = max(reach, hypot((cx - source.midX) * fullSquash, cy - source.midY))
    }
    val crossed = reach + (s.swell + COVER_ZONE) * fullSquash
    val settling = 1 - smooth(SETTLE_START, SETTLE_END, x)
    originX = source.midX
    originY = source.midY
    spine = startSpine * (1 - opening)
    radius = if (sinceLaunch < 0) startRadius + s.inhale * (1 - gathered) else startRadius + (crossed + OVERSHOOT - startRadius) * moved
    squash = 1 + (fullSquash - 1) * smooth(0.0, 0.5, moved)
    time = t
    swell = s.swell * smooth(0.0, 0.3, moved)
    val gatheredGlow = min(GATHERED_GLOW * max(s.gatherGlow, 0.0), 1.0)
    glow = if (sinceLaunch < 0) gatheredGlow * gathered else (gatheredGlow + (1 - gatheredGlow) * smooth(0.0, 0.07, x)) * settling
    val clearStart = startRadius - s.rimSoftness
    val clearEnd = reach + (s.rimSoftness + s.swell) * fullSquash
    clearRadius = clearStart + (clearEnd - clearStart) * s.rimEasing((sinceLaunch - s.linger) / max(s.rimFade, 0.01))
    hairClearRadius = clearStart + (clearEnd - clearStart) * s.rimEasing((sinceLaunch - s.linger - s.hairlineLinger) / max(s.hairlineFade, 0.01))
    rim = s.rimStrength * smooth(0.0, 0.1, x) * (1 - smooth(0.9, 1.0, t / s.duration))
    sparkle = smooth(0.0, max(s.anticipation, 0.001) * 0.35, t)
    flying = max(sinceLaunch, 0.0)
    launched = sinceLaunch > 0
  }

  companion object {
    const val COVER_ZONE = 60.0
    private const val ARC_SQUASH = 1.4
    private const val GATHERED_GLOW = 0.5
    private const val OVERSHOOT = 100.0
    private const val SETTLE_START = 0.2
    private const val SETTLE_END = 0.55

    fun smooth(a: Double, b: Double, x: Double): Double {
      val t = min(max((x - a) / (b - a), 0.0), 1.0)
      return t * t * (3 - 2 * t)
    }
  }
}

/** WaveRim: seven lumps that make the colour on the edges uneven. */
object WaveRim {
  const val count = 7

  fun lumps(time: Double, width: Double, height: Double, cornerRadius: Double, pulse: Double): FloatArray {
    val ex = max(width / 2 - cornerRadius, 0.0)
    val ey = max(height / 2 - cornerRadius, 0.0)
    val perimeter = 4 * (ex + ey + 0.5 * PI * cornerRadius)
    val out = FloatArray(count * 3)
    for (i in 0 until count) {
      val fi = i.toDouble()
      val a = hash(fi, 12.9898, 1.3)
      val b = hash(fi, 78.233, 4.1)
      val c = hash(fi, 39.425, 2.7)
      val centre = perimeter * (fi + 0.35 * a) / count + time * (-22 + 44 * b) * (1 + 2 * pulse)
      val length = perimeter / count * (0.2 + 0.25 * c)
      val breath = (0.25 + 0.3 * pulse) * sin(time * (1.1 + 1.2 * a) * (1 + pulse) + fi * 2.4)
      val depth = (0.2 + 2.6 * b) * (1 + breath)
      out[i * 3] = centre.toFloat()
      out[i * 3 + 1] = length.toFloat()
      out[i * 3 + 2] = depth.toFloat()
    }
    return out
  }

  private fun hash(i: Double, a: Double, b: Double): Double {
    val x = sin(i * a + b) * 43758.5453
    return x - floor(x)
  }
}

/** WaveParticle: a spark that falls toward the source, then rides out with the front. */
class WaveParticle(index: Int) {
  private val i = (index + 1).toDouble()
  private fun scatter(step: Double) = (i * step) % 1.0
  private val column = (scatter(0.7548776662) * 2 - 1) * 0.92
  private val pace = 0.35 + 0.72 * sqrt(scatter(0.8191725134))
  private val drift = 15 + 50 * scatter(0.6710436067)
  private val sway = 3 + 11 * scatter(0.4142135624)
  private val swayRate = 2.5 + 3.5 * scatter(0.7320508076)
  private val twinkleRate = 8 + 12 * scatter(0.2360679775)
  private val phase = scatter(0.6180339887) * 2 * PI
  private val life = WaveStyle.particleLife * (0.4 + 0.6 * scatter(0.6457513111))
  val size = WaveStyle.particleSize * (0.55 + 0.45 * scatter(0.5497004779).pow(2))
  private val brightness = 0.3 + 0.7 * scatter(0.6055512755)
  private val hop = 0.16 + 0.14 * scatter(0.1231056256)
  private val hopOffset = scatter(0.4721359550)

  fun opacity(front: WaveFront): Double {
    val twinkle = 0.5 + 0.5 * sin(front.time * twinkleRate + phase)
    val glint = brightness * (0.7 + 0.3 * twinkle)
    if (!front.launched) {
      val along = hopAt(front.time).second
      val hopping = WaveFront.smooth(0.0, 0.25, along) * (1 - WaveFront.smooth(0.55, 1.0, along))
      val ending = 1 - WaveFront.smooth(WaveStyle.anticipation * 0.75, WaveStyle.anticipation, front.time)
      return front.sparkle * hopping * ending * glint * WaveStyle.gatherDots
    }
    return (1 - WaveFront.smooth(life * 0.55, life, front.flying)) * glint
  }

  /** Where the spark is, dp from the top left. */
  fun point(front: WaveFront, source: Box): Pair<Double, Double> =
    if (front.launched) outflow(front, source) else inflow(front, source)

  private fun hopAt(time: Double): Pair<Double, Double> {
    val hops = time / hop + hopOffset
    val count = floor(hops)
    return count to hops - count
  }

  private fun inflow(front: WaveFront, source: Box): Pair<Double, Double> {
    val (count, along) = hopAt(front.time)
    val half = source.width / 2
    val face = source.height / 2
    val x = (hash(i, count, 1.0) * 2 - 1) * (half + 24)
    val nearest = WaveStyle.particleGap + WaveStyle.particleFlight
    val away = nearest + max(WaveStyle.particleReach - nearest, 0.0) * hash(i, count, 2.0)
    val startX = source.midX + x
    val startY = source.midY - (face + away)
    val corner = min(source.width, source.height) / 2
    val targetX = source.midX + min(max(x, -half + corner), half - corner)
    val targetY = source.midY - face
    val dx = targetX - startX
    val dy = targetY - startY
    val length = max(hypot(dx, dy), 1.0)
    val travelled = WaveStyle.particleFlight * along * along
    return (startX + dx / length * travelled) to (startY + dy / length * travelled)
  }

  private fun outflow(front: WaveFront, source: Box): Pair<Double, Double> {
    val start = min(source.width, source.height) / 2
    val spread = WaveStyle.particleSpread
    val widening = 1 + 0.22 * front.moved * pace * spread
    val swaying = sway * sin(swayRate * front.flying + phase) * min(front.flying / 0.25, 1.0)
    val x = column * source.width / 2 * widening + swaying
    val height = (start + (front.radius - start) * pace * spread) * (1 - 0.3 * column * column) + drift * front.flying * spread
    return (front.originX + x) to (front.originY - height)
  }

  private fun hash(i: Double, n: Double, salt: Double): Double {
    val x = sin(i * 12.9898 + n * 78.233 + salt * 37.719) * 43758.5453
    return x - floor(x)
  }
}
