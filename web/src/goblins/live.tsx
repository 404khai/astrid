import { useEffect, useRef, useState } from "react";
import { Astrid, type Shape } from "../studio/Astrid";
import { FORMS, type Corners, type Form } from "../studio/forms";
import { MOODS, moodLabel, parseStops, type Mood } from "../studio/faces";
import "../studio/mark.css";

/**
 * AstroNauts (called goblins in code and storage) for the desktop app's pages.
 * An AstroNaut is a mark from the AstroNauts studio (../studio, from
 * github.com/404khai/goblins v2, MIT; docs › AstroNauts).
 * The app sends each goblin as `{ id, name, character }`, where `character` is
 * its `astrid/mark@1` object.
 */
export type Mark = {
  name?: string; mood?: string; form?: string; corners?: string; shape?: string;
  top?: string | string[]; bottom?: string | string[]; arch?: string | string[]; eyes?: string | string[]; middle?: string | string[];
};
export type Item = { id: string; name: string; character: Mark };
export type { Mood };
export { moodLabel };

const stops = (v: unknown, fallback: string[]) => (Array.isArray(v) ? parseStops(v.join(",")) : typeof v === "string" ? parseStops(v) : null) ?? fallback;
const ASTRID = ["#4459F9", "#1A32EC", "#4459F9"];

/** A mark's drawing props, with anything missing or unknown filled in. */
export function look(c: Mark) {
  const top = stops(c.top ?? c.arch, ASTRID);
  return {
    top,
    bottom: stops(c.bottom ?? c.arch, top),
    eyes: stops(c.eyes, ["#00F7D5"])[0],
    middle: stops(c.middle, ["#121212"]),
    form: (FORMS as readonly string[]).includes(c.form ?? "") ? (c.form as Form) : "classic",
    corners: (c.corners === "sharp" ? "sharp" : "curved") as Corners,
    shape: (c.shape === "full" || c.shape === "dual" ? c.shape : "arches") as Shape,
  };
}

/** Moods the studio has since renamed or dropped, as the nearest it has. */
const ALIASES: Record<string, Mood> = { talking: "voice", announcing: "voice", jotting: "posting", drawing: "posting" };
export const moodOf = (m?: string): Mood => ((MOODS as readonly string[]).includes(m ?? "") ? (m as Mood) : ALIASES[m ?? ""] ?? "idle");

/** Every mood a goblin has. */
export const ANIMATIONS: readonly Mood[] = MOODS;

/**
 * A live goblin: it blinks and plays its mood's motion. Clicking it shows the
 * next of every mood in turn, then it settles back on `expression`.
 */
export function LiveGoblin({ item, size, expression, title }: { item: Item; size: number; expression?: string; framing?: "full" | "head"; title?: string }) {
  const [step, setStep] = useState(-1);
  const timer = useRef<number>(undefined);
  useEffect(() => () => clearTimeout(timer.current), []);
  const play = () => {
    const next = (step + 1) % ANIMATIONS.length;
    setStep(next);
    clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setStep(-1), 4000);
  };
  const shown = step >= 0 ? ANIMATIONS[step] : null;
  const mood = shown ?? moodOf(expression ?? item.character.mood);
  return (
    <button type="button" className="live-goblin" onClick={play} title={shown ? `${item.name}: ${shown}` : title ?? item.name} aria-label={`${item.name}. Click to see the next mood.`}>
      {/* A mark fills its box more than a 3D head did: draw it at 80%, centred in the same space. */}
      <span style={{ display: "grid", placeItems: "center", width: size, height: size }}>
        <Astrid {...look(item.character)} mood={mood} size={Math.round(size * 0.8)} blink animate />
      </span>
    </button>
  );
}

/** A still goblin, for small spots. */
export function FlatGoblin({ item, size, expression }: { item: Item; size: number; expression?: string }) {
  return <Astrid {...look(item.character)} mood={moodOf(expression ?? item.character.mood)} size={size} />;
}
