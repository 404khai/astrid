import { useEffect, useRef, useState } from "react";
import { Goblin3D, GoblinAvatar, EXPRESSION_IDS, MORPH_IDS, type ExpressionId, type Goblin, type MorphId } from "../goblin";

/**
 * Goblins (github.com/404khai/goblins, MIT, vendored in ../goblin) for the
 * desktop app's pages. The app sends each goblin as `{ id, name, character }`,
 * where `character` is its goblins/v1 recipe.
 */
export type Item = { id: string; name: string; character: Goblin };

export const recipe = (item: Item): Goblin => ({ ...item.character, id: item.id, name: item.name, extras: item.character.extras ?? [] });

/** Every animation the library has: its fifteen moods, then its fifteen spells. */
export const ANIMATIONS: ({ expression: ExpressionId } | { morph: MorphId })[] = [
  ...EXPRESSION_IDS.map((expression) => ({ expression })),
  ...MORPH_IDS.map((morph) => ({ morph })),
];

/**
 * A live 3D goblin. Clicking it plays the next of every mood and spell in turn
 * (a short show of the whole library), then it settles back on `expression`.
 */
export function LiveGoblin({ item, size, expression, morph, framing = "head", title }: { item: Item; size: number; expression?: ExpressionId; morph?: MorphId | null; framing?: "full" | "head"; title?: string }) {
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
  const expr = shown && "expression" in shown ? shown.expression : expression;
  const spell = shown ? ("morph" in shown ? shown.morph : null) : morph ?? null;
  return (
    <button type="button" className="live-goblin" onClick={play} title={shown ? ("morph" in shown ? `${item.name}: ${shown.morph}` : `${item.name}: ${shown.expression}`) : title ?? item.name} aria-label={`${item.name}. Click to see the next animation.`}>
      <Goblin3D goblin={recipe(item)} expression={expr} morph={spell} size={size} framing={framing} />
    </button>
  );
}

/** A flat goblin: cheap, for small spots. */
export function FlatGoblin({ item, size, expression }: { item: Item; size: number; expression?: ExpressionId }) {
  return <GoblinAvatar goblin={recipe(item)} expression={expression ?? "idle"} size={size} hands={false} props={false} idPrefix={`f${item.id.replace(/[^a-z0-9]/gi, "")}${size}`} />;
}
