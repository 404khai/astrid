// Ported from Libraries.dev Distortion Waves (DistortionWavesKit),
// https://github.com/Jakubantalik/Libraries.dev, commit 387b049, by way of
// presence's Metal copy (WaveBandShader.swift). MIT License, Copyright (c)
// 2026 Jakub Antalik: see apps/presence-mac/ThirdParty/distortion-waves/LICENSE.
// The band shader in AGSL for Android 13+, for the vertical wave only (no ring,
// no hold, no lean). Array lookups go through loops, as AGSL indexes uniform
// arrays only by loop counters.
package dev.astrid.overlay

const val WAVE_BAND_AGSL = """
uniform float scale;
uniform float2 size;
uniform float2 origin;
uniform float spine;
uniform float radius;
uniform float squash;
uniform float time;
uniform float swell;
uniform float glow;
uniform float clearRadius;
uniform float rim;
uniform float hairClear;
uniform float cornerRadius;
uniform float3 palette[9];
uniform float3 lumps[7];

const float PI = 3.14159265;
const float lead = 130.0;
const float tail = 300.0;
const float strength = 0.92;
const float rimWidth = 14.0;
const float rimSoftness = 343.0;
const float unevenness = 1.0;
const float pulse = 0.6;
const float hairWidth = 1.5;
const float hairOpacity = 0.9;
const float hairWhite = 0.95;

float sq(float x) { return x * x; }

float3 oklabToSRGB(float3 c) {
  float l = c.x + 0.3963377774 * c.y + 0.2158037573 * c.z;
  float m = c.x - 0.1055613458 * c.y - 0.0638541728 * c.z;
  float s = c.x - 0.0894841775 * c.y - 1.2914855480 * c.z;
  l = l * l * l; m = m * m * m; s = s * s * s;
  float3 lin = clamp(float3(
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
  ), 0.0, 1.0);
  return mix(12.92 * lin, 1.055 * pow(lin, float3(1.0 / 2.4)) - 0.055, step(float3(0.0031308), lin));
}

float continuousRectSDF(float2 p, float2 halfSize, float r) {
  float a = min(r * 1.528665, min(halfSize.x, halfSize.y));
  float2 q = abs(p) - (halfSize - a);
  if (q.x <= 0.0 || q.y <= 0.0) {
    return max(abs(p.x) - halfSize.x, abs(p.y) - halfSize.y);
  }
  float n = 3.26;
  float2 u = q / a;
  float norm = pow(pow(u.x, n) + pow(u.y, n), 1.0 / n);
  float2 g = pow(u / max(norm, 1e-4), float2(n - 1.0));
  return (norm - 1.0) * a / max(length(g), 1e-3);
}

float2 perimeterAt(float2 p, float2 halfSize, float r) {
  float ex = max(halfSize.x - r, 0.0);
  float ey = max(halfSize.y - r, 0.0);
  float arc = 0.5 * PI * r;
  float P = 4.0 * (ex + ey + arc);
  float ax = abs(p.x), ay = abs(p.y);
  if (ax > ex && ay > ey) {
    float sx = p.x >= 0.0 ? 1.0 : -1.0;
    float sy = p.y >= 0.0 ? 1.0 : -1.0;
    float2 v = p - float2(sx * ex, sy * ey);
    float a; float base;
    if (p.x >= 0.0 && p.y < 0.0) { a = atan(v.x, -v.y); base = ex; }
    else if (p.x >= 0.0) { a = atan(v.y, v.x); base = ex + arc + 2.0 * ey; }
    else if (p.y >= 0.0) { a = atan(-v.x, v.y); base = 3.0 * ex + 2.0 * arc + 2.0 * ey; }
    else { a = atan(-v.y, -v.x); base = 3.0 * ex + 3.0 * arc + 4.0 * ey; }
    return float2(base + r * clamp(a, 0.0, 0.5 * PI), P);
  }
  bool side = ax > ex ? true : (ay > ey ? false : (halfSize.x - ax) < (halfSize.y - ay));
  if (side) {
    return float2(p.x >= 0.0 ? ex + arc + (p.y + ey) : 3.0 * ex + 3.0 * arc + 2.0 * ey + (ey - p.y), P);
  }
  return float2(p.y < 0.0 ? (p.x >= 0.0 ? p.x : P + p.x) : ex + 2.0 * arc + 2.0 * ey + (ex - p.x), P);
}

float rimThickness(float s, float P) {
  float t = 0.08;
  for (int i = 0; i < 7; i++) {
    float ds = s - lumps[i].x;
    ds -= P * floor(ds / P + 0.5);
    t += lumps[i].z * exp(-sq(ds / lumps[i].y));
  }
  return t / 1.0498;
}

float dither(float2 p) {
  return fract(52.9829189 * fract(dot(p, float2(0.06711056, 0.00583715)))) - 0.5;
}

float3 paletteAt(float u) {
  float x = fract(u) * 7.0;
  float i = min(floor(x), 6.0);
  float j = mod(i + 1.0, 7.0);
  float m = fract(x);
  m = m * m * (3.0 - 2.0 * m);
  float3 a = palette[0];
  float3 b = palette[0];
  for (int k = 0; k < 7; k++) {
    if (float(k) == i) a = palette[k];
    if (float(k) == j) b = palette[k];
  }
  return mix(a, b, m);
}

half4 main(float2 fragCoord) {
  float2 p = fragCoord / scale;
  float2 c = p - size * 0.5;
  float inset = max(-continuousRectSDF(c, size * 0.5, cornerRadius), 0.0);
  bool nearEdge = rim > 0.0 && inset < max(rimWidth * 2.2 * (1.0 + 1.9 * unevenness), hairWidth * 0.5 + 0.75);
  if (glow <= 0.0 && !nearEdge) return half4(0.0);

  // The field: a pill around the source that opens into a bowed arc.
  float2 q = p - origin;
  float mx = q.x * squash;
  float2 v = float2(max(abs(mx) - spine, 0.0), q.y);
  float l = max(length(v), 1e-3);
  float sgn = q.x >= 0.0 ? 1.0 : -1.0;
  float2 grad = float2(v.x * sgn * squash, v.y) / l;
  float g = max(length(grad), 1e-3);
  float sb = mx * 0.0058;
  float bend = 0.62 * sin(sb + time * 1.1) + 0.38 * sin(sb * 2.1 - time * 1.5 + 1.7);
  float d = (l - radius) / g + swell * bend;

  float along = q.x * 0.0068;
  float lobes = 0.5 + 0.5 * sin(along * 1.3 + time * 1.4) * cos(along * 0.6 - time * 0.7 + 1.2);
  float bandA = 0.0;
  if (glow > 0.0 && d < lead) {
    if (d >= 0.0) {
      float k = 1.0 - d / lead;
      bandA = k * k * k * (k * (k * 6.0 - 15.0) + 10.0);
    } else {
      bandA = pow(1.0 - clamp(-d / (tail * mix(0.6, 1.3, lobes)), 0.0, 1.0), 2.3);
    }
    bandA *= glow * strength * mix(0.6, 1.0, exp(-inset / 140.0));
  }

  float around = atan(c.y / size.y, c.x / size.x);
  float rimA = 0.0;
  float hairA = 0.0;
  float shimmer = 0.5;
  if (nearEdge) {
    float2 edge = perimeterAt(c, size * 0.5, cornerRadius);
    float thickness = max(1.0 + (rimThickness(edge.x, edge.y) - 1.0) * unevenness, 0.04);
    float hug = exp(-sq(inset / (rimWidth * thickness)));
    float reached = 1.0 - smoothstep(-lead * 0.7, lead * 0.5, d);
    float kept = smoothstep(-rimSoftness, rimSoftness, d + (radius - clearRadius) / g);
    shimmer = 0.5 + 0.5 * sin(time * 3.4 - edge.x / edge.y * 4.0 * PI);
    rimA = hug * reached * kept * rim * mix(1.0, 0.7 + 0.3 * shimmer, pulse);
    float line = 1.0 - smoothstep(hairWidth * 0.5, hairWidth * 0.5 + 0.75, inset);
    float hairKept = smoothstep(-rimSoftness, rimSoftness, d + (radius - hairClear) / g);
    hairA = line * reached * hairKept * rim * hairOpacity
      * mix(0.5, 1.0, smoothstep(0.1, 1.5, thickness))
      * mix(1.0, 0.55 + 0.45 * shimmer, pulse);
  }

  float a = 1.0 - (1.0 - bandA) * (1.0 - rimA);
  float total = 1.0 - (1.0 - a) * (1.0 - hairA);
  if (total <= 0.0005) return half4(0.0);

  float flow = 0.62 + along * 0.2 + time * 0.13 + d * 0.0015 + 0.05 * sin(q.y * 0.009 + time);
  float3 lab = paletteAt(flow);
  float3 warm = palette[7];
  float3 cool = palette[8];
  lab = mix(lab, warm, 0.5 * exp(-sq((d - 30.0) / 50.0)));
  lab = mix(lab, cool, 0.5 * exp(-sq((d + 84.0) / 72.0)));
  lab.yz *= 1.22;
  lab = mix(lab, float3(0.99, 0.0, 0.0), 0.7 * exp(-sq((d + 8.0) / 26.0)));

  float3 rimLab = paletteAt(around / (2.0 * PI) + time * (0.12 + 0.25 * pulse) + inset * 0.0012);
  rimLab.yz *= 1.1;
  lab = mix(lab, rimLab, rimA / (rimA + bandA + 1e-4));

  float3 rgb = oklabToSRGB(lab);
  if (hairA > 0.0) {
    float3 hair = mix(oklabToSRGB(rimLab), float3(1.0), hairWhite * (0.5 + 0.5 * shimmer));
    rgb = (rgb * a * (1.0 - hairA) + hair * hairA) / total;
  }
  float noise = dither(p) * 2.0 / 255.0;
  return half4(half3(clamp(rgb * total + noise, 0.0, 1.0)), half(clamp(total + noise, 0.0, 1.0)));
}
"""
