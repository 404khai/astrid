// Astrid studio (github.com/404khai/goblins v2, MIT, ported into the docs' AstroNauts page): three bays. Left is identity, saved marks and the log; the middle is the mark on its ground,
// eyes tracking the pointer; the right bay holds mood, shape, arch colours, eyes, ground and export.
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Astrid, type Shape } from "./Astrid";
import { FORMS, FORM_LABEL, type Corners, type Form } from "./forms";
import { MOODS, PRESETS, formatStops, moodLabel, parseStops, type Mood } from "./faces";

interface Unit { name: string; mood: Mood; form: Form; corners: Corners; shape: Shape; top: string[]; bottom: string[]; linked: boolean; eyes: string; middle: string[]; ground: string | null }
const DEFAULT: Unit = { name: "Astrid", mood: "idle", form: "classic", corners: "curved", shape: "arches", top: ["#4459F9", "#1A32EC", "#4459F9"], bottom: ["#4459F9", "#1A32EC", "#4459F9"], linked: true, eyes: "#00F7D5", middle: ["#121212"], ground: "#121212" };
const GROUNDS: { label: string; value: string | null }[] = [{ label: "Black", value: "#121212" }, { label: "Paper", value: "#EEEEEE" }, { label: "None", value: null }];
const STORE = "astrid.studio.v1";
const SHAPE_LABEL: Record<Shape, string> = { arches: "Arches", full: "Full", dual: "Dual" };
const now = () => new Date().toTimeString().slice(0, 8);

function sanitize(v: unknown): Unit {
  const o = (v && typeof v === "object" ? v : {}) as Record<string, unknown>;
  const stops = (x: unknown, d: string[]) => (Array.isArray(x) ? parseStops(x.join(",")) : typeof x === "string" ? parseStops(x) : null) ?? d;
  const hex = (x: unknown, d: string) => (typeof x === "string" && parseStops(x)?.length === 1 ? parseStops(x)![0] : d);
  const top = stops(o.top ?? o.arch, DEFAULT.top);
  return {
    name: typeof o.name === "string" ? o.name.slice(0, 16) : DEFAULT.name,
    mood: MOODS.includes(o.mood as Mood) ? (o.mood as Mood) : "idle",
    form: FORMS.includes(o.form as Form) ? (o.form as Form) : "classic",
    corners: o.corners === "sharp" ? "sharp" : "curved",
    shape: o.shape === "full" || o.shape === "dual" ? o.shape : "arches",
    middle: stops(o.middle, DEFAULT.middle),
    top, bottom: stops(o.bottom ?? o.arch, top),
    linked: o.linked !== false && o.bottom === undefined ? true : o.linked === true,
    eyes: hex(o.eyes, DEFAULT.eyes),
    ground: o.ground === null ? null : hex(o.ground, DEFAULT.ground!),
  };
}
const toJSON = (u: Unit) => JSON.stringify({ $schema: "astrid/mark@1", name: u.name, mood: u.mood, form: u.form, corners: u.corners, shape: u.shape, top: formatStops(u.top), bottom: formatStops(u.bottom), eyes: formatStops([u.eyes]), middle: formatStops(u.middle), ground: u.ground && formatStops([u.ground]) }, null, 2);
function load(): Unit {
  // Share links open the docs' AstroNauts page: #astronauts:m=<mark JSON>
  try { const at = location.hash.indexOf("m="); if (at >= 0) return sanitize(JSON.parse(decodeURIComponent(location.hash.slice(at + 2)))); } catch {}
  try { return sanitize(JSON.parse(localStorage.getItem(STORE) ?? "null") ?? DEFAULT); } catch { return DEFAULT; }
}
function download(blob: Blob, name: string) {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

function Corners() {
  return <><i className="corner tl" /><i className="corner tr" /><i className="corner bl" /><i className="corner br" /></>;
}
function Head({ index, title, aside }: { index: string; title: string; aside?: ReactNode }) {
  return <div className="bay-head"><span className="idx">{index}</span><h2>{title}</h2><span className="rule" />{aside && <span className="aside">{aside}</span>}</div>;
}

/** A stops field in the "F94447, EC1A1D, F94447" format, with a picker per stop. */
function StopsField({ id, label, stops, onChange, max = 3 }: { id: string; label: string; stops: string[]; onChange: (s: string[]) => void; max?: number }) {
  const [draft, setDraft] = useState(formatStops(stops));
  const [bad, setBad] = useState(false);
  useEffect(() => { setDraft(formatStops(stops)); setBad(false); }, [stops.join()]);
  return (
    <div className="stops">
      <label className="lbl" htmlFor={id}>{label}</label>
      <div className="stops-row">
        {stops.map((c, i) => (
          <label key={i} className="stop" style={{ background: c }} title={`Stop ${i + 1}: ${c}`}>
            <input type="color" value={c.toLowerCase()} aria-label={`${label} stop ${i + 1}`} onChange={(e) => onChange(stops.map((s, j) => (j === i ? e.target.value.toUpperCase() : s)))} />
          </label>
        ))}
        <input id={id} className={`hex${bad ? " bad" : ""}`} value={draft} spellCheck={false} aria-invalid={bad}
          onChange={(e) => {
            setDraft(e.target.value);
            const p = parseStops(e.target.value);
            setBad(!p || p.length > max);
            if (p && p.length <= max) onChange(p);
          }} />
      </div>
    </div>
  );
}

export default function App() {
  const [u, setU] = useState<Unit>(load);
  const [saved, setSaved] = useState<Unit[]>(() => { try { return (JSON.parse(localStorage.getItem(STORE + ".saved") ?? "[]") as unknown[]).map(sanitize); } catch { return []; } });
  const [log, setLog] = useState([{ t: now(), msg: "studio online" }]);
  const [look, setLook] = useState({ x: 0, y: 0 });
  const [paste, setPaste] = useState("");
  const exportRef = useRef<HTMLDivElement>(null);
  const push = (msg: string) => setLog((l) => [{ t: now(), msg }, ...l].slice(0, 7));
  const set = (patch: Partial<Unit>, msg?: string) => { setU((x) => ({ ...x, ...patch })); if (msg) push(msg); };

  useEffect(() => { try { localStorage.setItem(STORE, JSON.stringify(u)); } catch {} }, [u]);
  useEffect(() => { try { localStorage.setItem(STORE + ".saved", JSON.stringify(saved)); } catch {} }, [saved]);

  const setTop = (top: string[]) => set(u.linked ? { top, bottom: top } : { top });
  const cycle = (d: number) => { const m = MOODS[(MOODS.indexOf(u.mood) + d + MOODS.length) % MOODS.length]; set({ mood: m }, `mood → ${m}`); };
  const shuffle = () => {
    const p = PRESETS[Math.floor(Math.random() * PRESETS.length)];
    const pick = <T,>(list: readonly T[]) => list[Math.floor(Math.random() * list.length)];
    const m = pick(MOODS), form = pick(FORMS), shape = pick(["arches", "full", "dual"] as Shape[]), corners = pick(["curved", "sharp"] as Corners[]);
    set({ top: parseStops(p.arch)!, bottom: parseStops(p.arch)!, linked: true, eyes: parseStops(p.eyes)![0], mood: m, form, shape, corners }, `shuffled → ${form} ${shape} ${corners} · ${p.name.toLowerCase()} · ${m}`);
  };
  const build = (text: string) => {
    try { const next = sanitize(JSON.parse(text)); setU(next); push(`built ${next.name.toLowerCase()}`); return true; }
    catch { push("not valid JSON: check quotes and commas"); return false; }
  };

  const svgText = (size: number) => {
    const svg = exportRef.current!.querySelector("svg")!.cloneNode(true) as SVGSVGElement;
    svg.setAttribute("width", String(size));
    svg.setAttribute("height", String(size));
    svg.removeAttribute("class");
    return new XMLSerializer().serializeToString(svg);
  };
  const file = (u.name.trim().toLowerCase().replace(/\W+/g, "-") || "astrid");
  const exportSvg = () => { download(new Blob([svgText(512)], { type: "image/svg+xml" }), `${file}.svg`); push("saved .svg"); };
  const exportPng = () => {
    const img = new Image();
    img.onload = () => {
      const c = document.createElement("canvas");
      c.width = c.height = 1024;
      c.getContext("2d")!.drawImage(img, 0, 0, 1024, 1024);
      c.toBlob((b) => b && download(b, `${file}.png`));
    };
    img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svgText(1024));
    push("saved .png @1024");
  };
  const copy = async (text: string, what: string) => { try { await navigator.clipboard.writeText(text); push(`copied ${what}`); } catch { push("clipboard blocked"); } };
  const share = () => copy(`${location.origin}${location.pathname}#astronauts:m=${encodeURIComponent(JSON.stringify(JSON.parse(toJSON(u))))}`, "share link");

  // R shuffles, ←/→ step through moods, mark JSON pasted outside a field is built
  const keys = useRef({ shuffle, cycle, build });
  keys.current = { shuffle, cycle, build };
  useEffect(() => {
    const typing = (el: EventTarget | null) => !!(el as HTMLElement)?.closest?.("input, textarea");
    const onKey = (e: KeyboardEvent) => {
      if (typing(e.target) || e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "r") keys.current.shuffle();
      if (e.key === "ArrowRight") keys.current.cycle(1);
      if (e.key === "ArrowLeft") keys.current.cycle(-1);
    };
    const onPaste = (e: ClipboardEvent) => {
      if (typing(e.target)) return;
      const text = e.clipboardData?.getData("text") ?? "";
      if (!text.trim().startsWith("{")) return;
      e.preventDefault();
      keys.current.build(text);
    };
    addEventListener("keydown", onKey);
    addEventListener("paste", onPaste);
    return () => { removeEventListener("keydown", onKey); removeEventListener("paste", onPaste); };
  }, []);

  const mark = { top: u.top, bottom: u.bottom, eyes: u.eyes, shape: u.shape, form: u.form, corners: u.corners, middle: u.middle };
  const accent = u.top[Math.floor(u.top.length / 2)];

  return (
    <div className="studio-page" style={{ ["--frame" as string]: accent, ["--eye" as string]: u.eyes }}>
      <header className="topbar">
        <a className="brand" href="#astronauts"><Astrid {...mark} mood={u.mood} size={34} /><span>Astrid</span><span className="crumb">/ studio</span></a>
        <span className="status"><i />{MOODS.length} moods · {PRESETS.length} palettes</span>
      </header>
      <main className="bench">
        <aside className="bay bay-left">
          <Corners />
          <Head index="01" title="Identity" aside={<span className="live">Live</span>} />
          <label className="name-field">
            <span className="lbl">Name</span>
            <input id="name" maxLength={16} spellCheck={false} value={u.name} onChange={(e) => set({ name: e.target.value })} onBlur={() => push(`renamed → ${u.name.toLowerCase()}`)} />
          </label>
          <dl className="spec">
            <div><dt>Shape</dt><dd>{FORM_LABEL[u.form]} · {SHAPE_LABEL[u.shape]} · {u.corners}</dd></div>
            {u.shape === "dual" && <div><dt>Middle</dt><dd>{formatStops(u.middle)}</dd></div>}
            <div><dt>Top</dt><dd>{formatStops(u.top)}</dd></div>
            <div><dt>Bottom</dt><dd>{formatStops(u.bottom)}</dd></div>
            <div><dt>Eyes</dt><dd><i className="dot" style={{ background: u.eyes }} />{formatStops([u.eyes])}</dd></div>
            <div><dt>Ground</dt><dd>{u.ground ? formatStops([u.ground]) : "None"}</dd></div>
            <div><dt>Mood</dt><dd>{moodLabel(u.mood)}</dd></div>
          </dl>

          <Head index="02" title="Saved" aside={`${saved.length}`} />
          {saved.length === 0 && <p className="blurb">Save a mark to keep it here.</p>}
          <ol className="roster">
            {saved.map((s, i) => (
              <li key={i}>
                <button type="button" onClick={() => { setU(s); push(`loaded ${s.name.toLowerCase()}`); }}>
                  <Astrid top={s.top} bottom={s.bottom} eyes={s.eyes} shape={s.shape} form={s.form} corners={s.corners} middle={s.middle} mood={s.mood} size={30} background={s.ground} />
                  <span className="who">{s.name}</span>
                  <span className="n">{moodLabel(s.mood)}</span>
                </button>
                <button type="button" className="x" aria-label={`Delete ${s.name}`} onClick={() => setSaved((l) => l.filter((_, j) => j !== i))}>×</button>
              </li>
            ))}
          </ol>
          <div className="row2">
            <button type="button" className="btn" onClick={() => { setSaved((l) => [u, ...l].slice(0, 24)); push(`saved ${u.name.toLowerCase()}`); }}>Save mark</button>
            <button type="button" className="btn ghost" onClick={() => { setU(DEFAULT); push("reset to astrid"); }}>Reset</button>
          </div>

          <Head index="03" title="Log" />
          <ol className="log" role="status">
            {log.map((l, i) => <li key={`${l.t}-${i}-${l.msg}`} style={{ opacity: 1 - i * 0.12 }}><span>{l.t}</span> {l.msg}</li>)}
          </ol>
        </aside>

        <section className="stage-col">
          <div className="stage" style={{ background: u.ground ?? undefined }} data-ground={u.ground ? undefined : "none"}
            onPointerMove={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              const nx = (e.clientX - r.left) / r.width - 0.5, ny = (e.clientY - r.top) / r.height - 0.5;
              setLook({ x: Math.max(-1, Math.min(1, nx * 2)) * 16, y: Math.max(-1, Math.min(1, ny * 2)) * 12 });
            }}
            onPointerLeave={() => setLook({ x: 0, y: 0 })}>
            <Astrid {...mark} mood={u.mood} size={560} look={look} blink animate title={`${u.name}, ${moodLabel(u.mood)}`} />
            <div className="hud hud-top"><span className="readout">{u.form.toUpperCase()} · {u.shape.toUpperCase()} · {u.mood.toUpperCase()}</span><span>{u.ground ? formatStops([u.ground]) : "TRANSPARENT"}</span></div>
            <p className="hud hud-bottom">The eyes follow your pointer · ← → change mood · R shuffles</p>
          </div>
          <div className="deck">
            <button type="button" className="btn ghost sq" onClick={() => cycle(-1)} aria-label="Previous mood">←</button>
            <button type="button" className="btn primary" onClick={shuffle}>Shuffle <kbd>R</kbd></button>
            <button type="button" className="btn ghost sq" onClick={() => cycle(1)} aria-label="Next mood">→</button>
            <span className="spacer" />
            <button type="button" className="btn" onClick={exportSvg}>SVG ↓</button>
            <button type="button" className="btn" onClick={exportPng}>PNG ↓</button>
            <button type="button" className="btn" onClick={share}>Share ⧉</button>
          </div>
          <div ref={exportRef} hidden><Astrid {...mark} mood={u.mood} size={512} background={u.ground} /></div>
        </section>

        <aside className="bay bay-right">
          <Corners />
          <Head index="04" title="Mood" aside="← →" />
          <div className="moods">
            {MOODS.map((m) => (
              <button key={m} id={`mood-${m}`} type="button" className="mood" aria-pressed={u.mood === m} onClick={() => set({ mood: m }, `mood → ${m}`)}>
                <Astrid {...mark} mood={m} size={40} animate={u.mood === m} />
                <span>{moodLabel(m)}</span>
              </button>
            ))}
          </div>

          <Head index="05" title="Shape" aside={`${FORMS.length} forms`} />
          <div className="forms">
            {FORMS.map((f) => (
              <button key={f} id={`form-${f}`} type="button" aria-pressed={u.form === f} onClick={() => set({ form: f }, `form → ${f}`)}>
                <Astrid {...mark} form={f} mood={u.mood} size={40} />
                <span>{FORM_LABEL[f]}</span>
              </button>
            ))}
          </div>
          <div className="seg wide" role="group" aria-label="Shape">
            {(["arches", "full", "dual"] as Shape[]).map((s) => (
              <button key={s} type="button" aria-pressed={u.shape === s} onClick={() => set({ shape: s }, `shape → ${s}`)}>
                <Astrid {...mark} mood={u.mood} shape={s} size={28} /> {SHAPE_LABEL[s]}
              </button>
            ))}
          </div>
          <div className="seg half" role="group" aria-label="Corners">
            {(["curved", "sharp"] as Corners[]).map((c) => (
              <button key={c} type="button" id={`corners-${c}`} aria-pressed={u.corners === c} onClick={() => set({ corners: c }, `corners → ${c}`)}>
                <Astrid {...mark} corners={c} mood={u.mood} size={28} /> {c === "curved" ? "Curved" : "Sharp"}
              </button>
            ))}
          </div>

          <Head index="06" title="Colour" aside={
            <button type="button" className="link" aria-pressed={u.linked} onClick={() => set(u.linked ? { linked: false } : { linked: true, bottom: u.top }, u.linked ? "arches split" : "arches linked")}>{u.linked ? "Split arches" : "Link arches"}</button>
          } />
          <StopsField id="arch-top" label={u.linked ? "Arches" : u.shape === "full" ? "Shape" : "Top arch"} stops={u.top} onChange={setTop} />
          {!u.linked && u.shape === "arches" && <StopsField id="arch-bottom" label="Bottom arch" stops={u.bottom} onChange={(bottom) => set({ bottom })} />}
          {u.shape === "dual" && <StopsField id="middle" label="Middle" stops={u.middle} onChange={(middle) => set({ middle })} />}
          <StopsField id="eyes" label="Eyes" stops={[u.eyes]} max={1} onChange={([eyes]) => set({ eyes })} />
          <span className="lbl">Palettes</span>
          <div className="palettes">
            {PRESETS.map((p) => {
              const arch = parseStops(p.arch)!, eyes = parseStops(p.eyes)![0];
              const on = formatStops(u.top) === formatStops(arch) && formatStops(u.bottom) === formatStops(arch) && u.eyes === eyes;
              return (
                <button key={p.id} type="button" aria-pressed={on} title={`${p.name}: ${p.arch} / ${p.eyes}`} onClick={() => set({ top: arch, bottom: arch, linked: true, eyes }, `palette → ${p.name.toLowerCase()}`)}>
                  <Astrid top={arch} bottom={arch} eyes={eyes} shape={u.shape} form={u.form} corners={u.corners} middle={u.middle} mood={u.mood} size={34} />
                  <span>{p.name}</span>
                </button>
              );
            })}
          </div>

          <Head index="07" title="Ground" />
          <div className="row2">
            <div className="seg" role="group" aria-label="Ground">
              {GROUNDS.map((g) => <button key={g.label} type="button" aria-pressed={u.ground === g.value} onClick={() => set({ ground: g.value }, `ground → ${g.label.toLowerCase()}`)}>{g.label}</button>)}
            </div>
            <label className="stop" style={{ background: u.ground ?? "transparent" }} title="Custom ground">
              <input type="color" aria-label="Custom ground" value={(u.ground ?? "#121212").toLowerCase()} onChange={(e) => set({ ground: e.target.value.toUpperCase() })} />
            </label>
          </div>

          <Head index="08" title="JSON" />
          <pre className="snippet"><code>{toJSON(u)}</code></pre>
          <div className="exp-grid">
            <button type="button" className="chip" onClick={() => copy(toJSON(u), "json")}>Copy JSON</button>
            <button type="button" className="chip" onClick={() => { download(new Blob([toJSON(u)], { type: "application/json" }), `${file}.json`); push("saved .json"); }}>JSON ↓</button>
            <button type="button" className="chip" onClick={() => copy(svgText(512), "svg markup")}>Copy SVG</button>
          </div>
          <label htmlFor="paste-json" className="lbl">Paste mark JSON</label>
          <textarea id="paste-json" className="paste" rows={3} spellCheck={false} placeholder='{ "top": "F94447, EC1A1D, F94447", "eyes": "F7C600" }' value={paste} onChange={(e) => setPaste(e.target.value)} />
          <button type="button" className="btn" disabled={!paste.trim()} onClick={() => build(paste) && setPaste("")}>Build from JSON</button>
        </aside>
      </main>
    </div>
  );
}
