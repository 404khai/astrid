import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { LiveGoblin, type Item } from "./live";
import "./goblins.css";

/**
 * The desktop sidebar's row of goblins, live in 3D. Clicking one picks it and
 * shows its next animation; the picked one thinks while Astrid works.
 */
type Incoming = { type: "goblins"; items: Item[]; selected: string | null } | { type: "state"; state: string };
const host = window as unknown as {
  astrid?: { receive: (m: Incoming) => void };
  webkit?: { messageHandlers?: { astrid?: { postMessage: (m: unknown) => void } } };
};

function Row() {
  const [items, setItems] = useState<Item[]>([]);
  const [picked, setPicked] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    host.astrid = {
      receive: (m) => {
        if (m.type === "goblins") {
          setItems(m.items);
          setPicked(m.selected);
        } else setBusy(m.state === "thinking");
      },
    };
    host.webkit?.messageHandlers?.astrid?.postMessage({ type: "ready" });
  }, []);
  return (
    <main className="row">
      {items.map((item) => (
        <span
          key={item.id}
          className={`slot${item.id === picked ? " on" : ""}`}
          onClickCapture={() => {
            setPicked(item.id);
            host.webkit?.messageHandlers?.astrid?.postMessage({ type: "select", id: item.id });
          }}
        >
          <LiveGoblin item={item} size={40} expression={item.id === picked && busy ? "thinking" : "idle"} />
        </span>
      ))}
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Row />
  </StrictMode>,
);
