import { useEffect, useRef, useState } from "react";
import { RuneIcon } from "runeicons-react";
import { AstridLogo } from "./AstridLogo";
import { BrandLink } from "./BrandMenu";
import { VerseText } from "./VerseText";

/** Navbar, footer and shared bits used by every Astrid web page. */
export const repo = "https://github.com/404khai/astrid";
export const doc = (path: string) => `${repo}/blob/main/${path}`;
export const gameplan = `${repo}/blob/v2/docs/plans/v2-gameplan.md`;

/** Section headings print in VerseText; "\n" keeps the designed line breaks. */
export function SectionTitle({ id, text }: { id?: string; text: string }) {
  return (
    <div className="h2" role="heading" aria-level={2} id={id}>
      <VerseText text={text} tone="dark" idle={false} seed={text.length * 31} />
    </div>
  );
}

export function Icon({ name = "arrow-right" }: { name?: string }) {
  return (
    <span className="icon" aria-hidden="true">
      <RuneIcon name={`arrows-${name}`} size={20} />
    </span>
  );
}
export function Action({
  children,
  href,
  secondary = false,
}: {
  children: React.ReactNode;
  href: string;
  secondary?: boolean;
}) {
  return (
    <a className={`action ${secondary ? "secondary" : ""}`} href={href}>
      {children}
      <Icon name={secondary ? "arrow-right" : "arrow-up-right"} />
    </a>
  );
}
export function Navigation() {
  const [open, setOpen] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const mobileMenu = useRef<HTMLDivElement>(null);
  const mobileTrigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open && !mobileOpen) return;
    const outside = (event: PointerEvent) => {
      if (!menu.current?.contains(event.target as Node)) setOpen(false);
      if (!mobileMenu.current?.contains(event.target as Node))
        setMobileOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
        if (mobileOpen) {
          setMobileOpen(false);
          mobileTrigger.current?.focus();
        } else trigger.current?.focus();
      }
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("keydown", escape);
    };
  }, [open, mobileOpen]);
  return (
    <header className="navbar">
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <div className="mobile-menu" ref={mobileMenu}>
        <button
          className="mobile-nav-box"
          ref={mobileTrigger}
          aria-expanded={mobileOpen}
          aria-controls="mobile-links"
          onClick={() => setMobileOpen(!mobileOpen)}
        >
          Menu
          <Icon name="chevron-down" />
        </button>
        {mobileOpen && (
          <nav
            className="mobile-dropdown"
            id="mobile-links"
            aria-label="Mobile navigation"
          >
            {[
              ["See it work", "/#demo"],
              ["Spatial", "/#spatial"],
              ["Gameplan", "/#gameplan"],
              ["Changelog", "/changelog.html"],
              ["Docs", "/docs.html"],
              ["Build notes", doc("docs/development.md")],
              ["Contributing", doc("AGENTS.md")],
              ["Issues", `${repo}/issues`],
            ].map(([label, href]) => (
              <a key={label} href={href} onClick={() => setMobileOpen(false)}>
                {label}
                <Icon />
              </a>
            ))}
          </nav>
        )}
      </div>
      <nav className="nav-box nav-left" aria-label="Sections">
        <a href="/#spatial">Spatial</a>
        <a href="/#gameplan">Gameplan</a>
        <a href="/changelog.html">Changelog</a>
      </nav>
      <BrandLink className="brand" href="/" label="Astrid home">
        <AstridLogo animated tracking />
      </BrandLink>
      <a className="mobile-nav-box mobile-github" href={repo}>
        GitHub
        <Icon name="arrow-up-right" />
      </a>
      <nav className="nav-box nav-right" aria-label="Project">
        <a href="/docs.html">Docs</a>
        <a href={repo}>GitHub</a>
        <div className="more" ref={menu}>
          <button
            ref={trigger}
            onClick={() => setOpen(!open)}
            aria-expanded={open}
            aria-controls="more-links"
          >
            More
            <Icon name="chevron-down" />
          </button>
          {open && (
            <div className="dropdown" id="more-links">
              <a
                href={doc("docs/development.md")}
                onClick={() => setOpen(false)}
              >
                Build notes
                <Icon />
              </a>
              <a href={doc("AGENTS.md")} onClick={() => setOpen(false)}>
                Contributing
                <Icon />
              </a>
              <a href={`${repo}/issues`} onClick={() => setOpen(false)}>
                Issues
                <Icon />
              </a>
            </div>
          )}
        </div>
      </nav>
    </header>
  );
}
export function Footer() {
  return (
    <footer>
      <div className="footer-content">
        <div className="footer-callout">
          <div>
            <p className="eyebrow">A PERSONAL AGENT. BUILT IN THE OPEN.</p>
            <SectionTitle text={"See it. Understand it."} />
          </div>
          <Action href={repo}>Explore Astrid</Action>
        </div>
        <div className="footer-links">
          <div className="footer-brand">
            <AstridLogo />
            <p>
              A personal agent that sees what you see.
              <br />
              Points, sketches and explains.
            </p>
          </div>
          {[
            {
              title: "EXPLORE",
              links: [
                ["See it work", "/#demo"],
                ["How it helps", "/#help"],
                ["Spatial", "/#spatial"],
              ],
            },
            {
              title: "PROJECT",
              links: [
                ["GitHub", repo],
                ["Gameplan", gameplan],
                ["Docs", "/docs.html"],
                ["Changelog", "/changelog.html"],
              ],
            },
            {
              title: "PARTICIPATE",
              links: [
                ["Issues", `${repo}/issues`],
                ["Contributing", doc("AGENTS.md")],
                ["Build notes", doc("docs/development.md")],
              ],
            },
          ].map(({ title, links }) => (
            <nav key={title} aria-label={title}>
              <span className="eyebrow">{title}</span>
              {links.map(([label, href]) => (
                <a key={label} href={href}>
                  {label}
                </a>
              ))}
            </nav>
          ))}
        </div>
        <div className="footer-meta">
          <span>© 2026 ASTRID</span>
          <span>HELP THAT LIVES ON YOUR SCREEN.</span>
          <span>DESIGNED & BUILT BY 404KHAI</span>
        </div>
      </div>
      <div className="wordmark" aria-hidden="true">
        <VerseText text="ASTRID" tone="dark" seed={2026} />
      </div>
    </footer>
  );
}
