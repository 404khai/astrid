import {
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import {
  ArrowUpRight01Icon,
  Copy01Icon,
  PaintBoardIcon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import logoSvg from "../assets/astrid-logo.svg?raw";

export const BRAND_GUIDELINES = "/docs.html#brand";

type Item = {
  id: string;
  label: string;
  icon: IconSvgElement;
  href?: string;
  run: () => void | Promise<void>;
};

export function BrandLink({
  href,
  className,
  label,
  children,
}: {
  href: string;
  className: string;
  label: string;
  children: ReactNode;
}) {
  const link = useRef<HTMLAnchorElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const [copied, setCopied] = useState(false);

  const close = (refocus = false) => {
    setAt(null);
    setCopied(false);
    if (refocus) link.current?.focus();
  };

  const items: Item[][] = [
    [
      {
        id: "copy",
        label: copied ? "Copied" : "Copy logo as SVG",
        icon: Copy01Icon,
        run: async () => {
          try {
            await navigator.clipboard.writeText(logoSvg);
            setCopied(true);
            window.setTimeout(() => close(true), 700);
          } catch {
            close(true);
          }
        },
      },
      {
        id: "brand",
        label: "Go to brand guidelines",
        icon: PaintBoardIcon,
        run: () => {
          close();
          window.location.assign(BRAND_GUIDELINES);
        },
      },
    ],
    [
      {
        id: "tab",
        label: "Open link in new tab",
        icon: ArrowUpRight01Icon,
        href: link.current?.href ?? href,
        run: () => {
          window.setTimeout(() => close(true));
        },
      },
    ],
  ];

  const open = (event: ReactMouseEvent<HTMLAnchorElement>) => {
    event.preventDefault();
    const bounds = event.currentTarget.getBoundingClientRect();
    const keyboard = event.clientX === 0 && event.clientY === 0;
    setCopied(false);
    setAt(
      keyboard
        ? { x: bounds.left, y: bounds.bottom + 6 }
        : { x: event.clientX, y: event.clientY },
    );
  };

  useEffect(() => {
    const element = menu.current;
    if (!at || !element) return;
    const { width, height } = element.getBoundingClientRect();
    const x = Math.max(8, Math.min(at.x, window.innerWidth - width - 8));
    const y = Math.max(8, Math.min(at.y, window.innerHeight - height - 8));
    element.style.left = `${x}px`;
    element.style.top = `${y}px`;
    element.querySelector<HTMLElement>("[role=menuitem]")?.focus();
  }, [at]);

  useEffect(() => {
    if (!at) return;
    const outside = (event: PointerEvent) => {
      if (!menu.current?.contains(event.target as Node)) close();
    };
    const dismiss = () => close();
    document.addEventListener("pointerdown", outside);
    window.addEventListener("blur", dismiss);
    window.addEventListener("resize", dismiss);
    window.addEventListener("scroll", dismiss, true);
    return () => {
      document.removeEventListener("pointerdown", outside);
      window.removeEventListener("blur", dismiss);
      window.removeEventListener("resize", dismiss);
      window.removeEventListener("scroll", dismiss, true);
    };
  }, [at]);

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const entries = Array.from(
      menu.current?.querySelectorAll<HTMLElement>("[role=menuitem]") ?? [],
    );
    const index = entries.indexOf(document.activeElement as HTMLElement);
    const move = (next: number) => {
      event.preventDefault();
      entries[(next + entries.length) % entries.length]?.focus();
    };
    if (event.key === "ArrowDown") move(index + 1);
    else if (event.key === "ArrowUp") move(index - 1);
    else if (event.key === "Home") move(0);
    else if (event.key === "End") move(entries.length - 1);
    else if (event.key === "Escape") {
      event.preventDefault();
      close(true);
    } else if (event.key === "Tab") close(true);
  };

  return (
    <>
      <a
        ref={link}
        className={className}
        href={href}
        aria-label={label}
        aria-haspopup="menu"
        onContextMenu={open}
      >
        {children}
      </a>
      {at &&
        createPortal(
          <div
            ref={menu}
            className="brand-menu"
            role="menu"
            aria-label="Astrid brand"
            style={{ left: at.x, top: at.y }}
            onKeyDown={onKeyDown}
            onContextMenu={(event) => event.preventDefault()}
          >
            {items.map((group, groupIndex) => (
              <div className="brand-menu-group" role="group" key={groupIndex}>
                {group.map((item) => {
                  const content = (
                    <>
                      <HugeiconsIcon
                        icon={item.icon}
                        size={18}
                        strokeWidth={1.6}
                        aria-hidden="true"
                      />
                      <span
                        aria-live={item.id === "copy" ? "polite" : undefined}
                      >
                        {item.label}
                      </span>
                    </>
                  );
                  return item.href ? (
                    <a
                      key={item.id}
                      role="menuitem"
                      tabIndex={-1}
                      href={item.href}
                      target="_blank"
                      rel="noopener noreferrer"
                      onClick={() => void item.run()}
                    >
                      {content}
                    </a>
                  ) : (
                    <button
                      key={item.id}
                      type="button"
                      role="menuitem"
                      tabIndex={-1}
                      onClick={() => void item.run()}
                    >
                      {content}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
}
