import { useEffect, useRef, useState } from "react";
import { EXPRESSION_IDS, type ExpressionId, type Goblin } from "@404khai/goblins/flat";
import { Goblin3D, GoblinAvatar } from "@404khai/goblins/react";

/**
 * Goblins (github.com/404khai/goblins v0.7.0, MIT, a dependency; expressions only, no spells for now) for the
 * desktop app's pages. The app sends each goblin as `{ id, name, character }`,
 * where `character` is its goblins/v1 recipe.
 */
export type Item = { id: string; name: string; character: Goblin };

export const recipe = (item: Item): Goblin => ({ ...item.character, id: item.id, name: item.name, extras: item.character.extras ?? [] });

/** Every expression the library has (spells are left out for now). */
export const ANIMATIONS: readonly ExpressionId[] = EXPRESSION_IDS;

/**
 * A live 3D goblin. Clicking it plays the next of every expression in turn
 * (a short show of the whole library), then it settles back on `expression`.
 */
export function LiveGoblin({ item, size, expression, framing = "head", title }: { item: Item; size: number; expression?: ExpressionId; framing?: "full" | "head"; title?: string }) {
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
  const expr = shown ?? expression;
  return (
    <button type="button" className="live-goblin" onClick={play} title={shown ? `${item.name}: ${shown}` : title ?? item.name} aria-label={`${item.name}. Click to see the next expression.`}>
      <Goblin3D goblin={recipe(item)} expression={expr} size={size} framing={framing} />
    </button>
  );
}

/** A flat goblin: cheap, for small spots. */
export function FlatGoblin({ item, size, expression }: { item: Item; size: number; expression?: ExpressionId }) {
  return <GoblinAvatar goblin={recipe(item)} expression={expression ?? "idle"} size={size} hands={false} props={false} idPrefix={`f${item.id.replace(/[^a-z0-9]/gi, "")}${size}`} />;
}
