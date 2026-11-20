import { Component, type ReactNode } from "react";
import "../bklit/bklit.css";
import { LineChart } from "@bklit/components/charts/line-chart";
import { Line } from "@bklit/components/charts/line";
import { AreaChart, Area } from "@bklit/components/charts/area-chart";
import { Grid } from "@bklit/components/charts/grid";
import { XAxis } from "@bklit/components/charts/x-axis";
import { ChartTooltip } from "@bklit/components/charts/tooltip";
import { BarChart } from "@bklit/components/charts/bar-chart";
import { Bar } from "@bklit/components/charts/bar";
import { BarXAxis } from "@bklit/components/charts/bar-x-axis";
import { PieChart } from "@bklit/components/charts/pie-chart";
import { PieSlice } from "@bklit/components/charts/pie-slice";
import { PieCenter } from "@bklit/components/charts/pie-center";
import { RingChart } from "@bklit/components/charts/ring-chart";
import { Ring } from "@bklit/components/charts/ring";
import { RingCenter } from "@bklit/components/charts/ring-center";
import { Gauge } from "@bklit/components/charts/gauge";
import { RadarChart } from "@bklit/components/charts/radar-chart";
import { RadarGrid } from "@bklit/components/charts/radar-grid";
import { RadarAxis } from "@bklit/components/charts/radar-axis";
import { RadarLabels } from "@bklit/components/charts/radar-labels";
import { RadarArea } from "@bklit/components/charts/radar-area";
import { FunnelChart } from "@bklit/components/charts/funnel-chart";
import { AreaSparkline } from "@bklit/components/charts/sparkline";

/**
 * A goblin's inline chart (ADR 0037 amendment 15), drawn with Bklit. The spec
 * comes from the model, so it is checked here again (the app checks it too)
 * and anything off is dropped rather than drawn. The goblin picks the kind:
 * - line / area: a trend over dates (labels are ISO dates)
 * - bar: comparing categories
 * - pie / ring: parts of a whole (ring for progress per item)
 * - funnel: stages that narrow
 * - gauge: one score out of 100
 * - radar: a few things rated 0–100 on the same axes
 * - sparkline: a tiny trend beside a sentence
 */
type Series = { name: string; values: number[] };
export type ChartSpec =
  | { kind: "line" | "area" | "bar" | "sparkline"; title?: string; labels: string[]; series: Series[] }
  | { kind: "pie" | "ring" | "funnel"; title?: string; items: { label: string; value: number; max?: number }[] }
  | { kind: "gauge"; title?: string; value: number; label?: string }
  | { kind: "radar"; title?: string; axes: string[]; series: Series[] };

const num = (v: unknown) => typeof v === "number" && Number.isFinite(v);
const str = (v: unknown, n = 40) => (typeof v === "string" ? v.slice(0, n) : "");

export function checkChart(raw: unknown): ChartSpec | null {
  if (!raw || typeof raw !== "object") return null;
  const r = raw as Record<string, unknown>;
  const title = str(r.title, 80) || undefined;
  const series = (n: number) =>
    Array.isArray(r.series)
      ? (r.series as Record<string, unknown>[]).slice(0, 4).map((s) => ({ name: str(s?.name) || "value", values: Array.isArray(s?.values) ? (s.values as unknown[]).slice(0, n).map((v) => (num(v) ? (v as number) : 0)) : [] }))
      : [];
  switch (r.kind) {
    case "line": case "area": case "bar": case "sparkline": {
      const labels = Array.isArray(r.labels) ? (r.labels as unknown[]).slice(0, 60).map((l) => str(l)) : [];
      const s = series(labels.length).filter((x) => x.values.length === labels.length);
      if (labels.length < 2 || !s.length) return null;
      if ((r.kind === "line" || r.kind === "area") && labels.some((l) => Number.isNaN(Date.parse(l)))) return { kind: "bar", title, labels, series: s };
      return { kind: r.kind, title, labels, series: s };
    }
    case "pie": case "ring": case "funnel": {
      const items = Array.isArray(r.items) ? (r.items as Record<string, unknown>[]).slice(0, 8).filter((i) => num(i?.value) && (i.value as number) >= 0).map((i) => ({ label: str(i.label) || "—", value: i.value as number, ...(num(i.max) && (i.max as number) > 0 ? { max: i.max as number } : {}) })) : [];
      return items.length >= 2 ? { kind: r.kind, title, items } : null;
    }
    case "gauge":
      return num(r.value) ? { kind: "gauge", title, value: Math.max(0, Math.min(100, r.value as number)), label: str(r.label) || undefined } : null;
    case "radar": {
      const axes = Array.isArray(r.axes) ? (r.axes as unknown[]).slice(0, 8).map((a) => str(a)) : [];
      const s = series(axes.length).filter((x) => x.values.length === axes.length);
      return axes.length >= 3 && s.length ? { kind: "radar", title, axes, series: s } : null;
    }
  }
  return null;
}

const COLORS = ["var(--chart-1)", "var(--chart-2)", "var(--chart-3)", "var(--chart-4)"];

function Draw({ spec }: { spec: ChartSpec }) {
  switch (spec.kind) {
    case "line":
    case "area": {
      const data = spec.labels.map((l, i) => Object.fromEntries([["date", new Date(l)], ...spec.series.map((s, k) => [`s${k}`, s.values[i]])]));
      const Chart = spec.kind === "line" ? LineChart : AreaChart;
      return (
        <Chart data={data} aspectRatio="2 / 1">
          <Grid horizontal />
          {spec.series.map((s, k) => (spec.kind === "line" ? <Line key={k} dataKey={`s${k}`} stroke={COLORS[k]} /> : <Area key={k} dataKey={`s${k}`} fill={COLORS[k]} fillOpacity={0.35} />))}
          <XAxis />
          <ChartTooltip />
        </Chart>
      );
    }
    case "bar": {
      const data = spec.labels.map((l, i) => Object.fromEntries([["label", l], ...spec.series.map((s, k) => [s.name, s.values[i]])]));
      return (
        <BarChart data={data} xDataKey="label" aspectRatio="2 / 1">
          <Grid horizontal />
          {spec.series.map((s, k) => <Bar key={k} dataKey={s.name} fill={COLORS[k]} lineCap="round" />)}
          <BarXAxis />
          <ChartTooltip />
        </BarChart>
      );
    }
    case "sparkline": {
      const data = spec.series[0].values.map((v, i) => ({ i, v }));
      return <AreaSparkline data={data} dataKey="v" height={48} color="var(--chart-1)" />;
    }
    case "pie":
      return (
        <PieChart data={spec.items} size={200}>
          {spec.items.map((it, i) => <PieSlice index={i} key={it.label + i} />)}
          <PieCenter defaultLabel={spec.title ?? "Total"} />
        </PieChart>
      );
    case "ring":
      return (
        <RingChart data={spec.items.map((it) => ({ label: it.label, value: it.value, maxValue: it.max ?? Math.max(100, it.value) }))} size={200} strokeWidth={12}>
          {spec.items.map((it, i) => <Ring index={i} key={it.label + i} />)}
          <RingCenter defaultLabel={spec.title ?? "Total"} />
        </RingChart>
      );
    case "funnel":
      return <FunnelChart data={spec.items} />;
    case "gauge":
      return <Gauge value={spec.value} centerValue={spec.value} totalNotches={40} defaultLabel={spec.label ?? spec.title ?? "Score"} />;
    case "radar": {
      const metrics = spec.axes.map((a, i) => ({ key: `m${i}`, label: a }));
      const data = spec.series.map((s, k) => ({ label: s.name, color: COLORS[k], values: Object.fromEntries(s.values.map((v, i) => [`m${i}`, Math.max(0, Math.min(100, v))])) }));
      return (
        <RadarChart data={data} metrics={metrics} size={240}>
          <RadarGrid />
          <RadarAxis />
          <RadarLabels fontSize={10} offset={14} />
          {data.map((d, k) => <RadarArea key={d.label + k} index={k} />)}
        </RadarChart>
      );
    }
  }
}

class Guard extends Component<{ children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() { return this.state.failed ? <span className="chart-failed">Couldn't draw this chart.</span> : this.props.children; }
}

export function InlineChart({ spec }: { spec: ChartSpec }) {
  return (
    <figure className={`inline-chart ${spec.kind}`}>
      {spec.title && spec.kind !== "pie" && spec.kind !== "ring" && <figcaption>{spec.title}</figcaption>}
      <Guard><Draw spec={spec} /></Guard>
    </figure>
  );
}
