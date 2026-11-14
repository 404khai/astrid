import { createAvatar } from "@bible-strong/avatar-web";
import "./blobs.css";

/**
 * The desktop sidebar's row of blobs: little bots the user exported from Avatar
 * Lab (avatars.bible-strong.app) into Astrid's Blobs folder, drawn by the MIT
 * `@bible-strong/avatar-web` renderer. The app sends the characters; clicking
 * one picks it, and it gives a little bounce.
 */
type Blob = { id: string; name: string; character: unknown };
type Incoming = { type: "blobs"; items: Blob[]; selected: string | null } | { type: "state"; state: string };

const host = window as unknown as {
  astrid?: { receive: (m: Incoming) => void };
  webkit?: { messageHandlers?: { astrid?: { postMessage: (m: unknown) => void } } };
};
const root = document.getElementById("root")!;
let avatars: { id: string; el: HTMLButtonElement; avatar: ReturnType<typeof createAvatar> }[] = [];
let selected: string | null = null;

function select(id: string | null) {
  selected = id;
  for (const a of avatars) a.el.classList.toggle("on", a.id === id);
}

function render(items: Blob[], pick: string | null) {
  for (const a of avatars) a.avatar.destroy();
  avatars = [];
  root.replaceChildren();
  for (const item of items) {
    const el = document.createElement("button");
    el.type = "button";
    el.className = "blob";
    el.title = item.name;
    el.setAttribute("aria-label", item.name);
    root.append(el);
    try {
      const avatar = createAvatar(el, { character: item.character, size: 26, settleAfterMs: 5000, followPointer: true, ariaLabel: item.name });
      avatars.push({ id: item.id, el, avatar });
      el.addEventListener("click", () => {
        select(item.id);
        avatar.poke("bounce");
        avatar.setState("happy");
        host.webkit?.messageHandlers?.astrid?.postMessage({ type: "select", id: item.id });
      });
    } catch {
      // Not a character Avatar Lab exported: leave it out.
      el.remove();
    }
  }
  select(pick);
}

host.astrid = {
  receive: (m) => {
    if (m.type === "blobs") render(m.items, m.selected);
    // The picked blob reacts to what Astrid is doing.
    else if (m.type === "state") avatars.find((a) => a.id === selected)?.avatar.setState(m.state);
  },
};
host.webkit?.messageHandlers?.astrid?.postMessage({ type: "ready" });
