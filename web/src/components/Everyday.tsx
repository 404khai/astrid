import {
  AlarmClockIcon,
  Attachment01Icon,
  Calculator01Icon,
  Calendar03Icon,
  CodeIcon,
  Globe02Icon,
  Mail01Icon,
  PaintBoardIcon,
  Search01Icon,
  Settings01Icon,
  Timer02Icon,
  Video01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import type { ReactNode } from "react";
import { AstridCursor } from "./AstridCursor";
import { AstridLogo } from "./AstridLogo";

function Glyph({ icon, size = 18 }: { icon: IconSvgElement; size?: number }) {
  return <HugeiconsIcon icon={icon} size={size} strokeWidth={1.6} aria-hidden="true" />;
}

function Card({
  area,
  title,
  sub,
  centered = false,
  children,
}: {
  area: string;
  title: string;
  sub: string;
  centered?: boolean;
  children: ReactNode;
}) {
  return (
    <article className={`day-card day-${area} ${centered ? "centered" : ""}`}>
      <h3>{title}</h3>
      <p>{sub}</p>
      <div className="day-visual">{children}</div>
    </article>
  );
}

function MailVisual() {
  return (
    <div className="mail-visual" aria-hidden="true">
      <span className="mail-search">
        <Glyph icon={Search01Icon} size={14} /> lease renewal from my landlord
      </span>
      {[
        ["Re: Lease renewal, flat 4B", "Sarah · landlord", true],
        ["Renewal terms attached", "Sarah · landlord", true],
        ["Your October statement", "Bank", false],
      ].map(([subject, from, hit]) => (
        <span key={subject as string} className={`mail-row ${hit ? "hit" : ""}`}>
          <Glyph icon={Mail01Icon} size={16} />
          <b>{subject}</b>
          <small>{from}</small>
          {hit && subject === "Renewal terms attached" ? <Glyph icon={Attachment01Icon} size={13} /> : null}
        </span>
      ))}
      <span className="envelope" />
    </div>
  );
}

function TimerVisual() {
  return (
    <div className="timer-visual" aria-hidden="true">
      <div className="timer-ring">
        <span>
          <b>07:59</b>
          <small>PASTA</small>
        </span>
      </div>
      <div className="timer-list">
        <span>
          <Glyph icon={AlarmClockIcon} size={16} />
          <b>Alarm</b> 7:00 AM tomorrow <i className="toggle on" />
        </span>
        <span>
          <Glyph icon={Timer02Icon} size={16} />
          <b>Timer</b> pasta · 8 min <i className="toggle on" />
        </span>
        <span>
          <Glyph icon={Calendar03Icon} size={16} />
          <b>Remind</b> call mum at 6 <i className="toggle" />
        </span>
      </div>
      <span className="said">“set a timer for the pasta, eight minutes”</span>
    </div>
  );
}

function EyesVisual() {
  return (
    <div className="eyes-visual" aria-hidden="true">
      <span className="eyes-mark">
        <AstridLogo tracking />
      </span>
      <span className="eyes-hint">MOVE YOUR MOUSE</span>
    </div>
  );
}

const apps: [IconSvgElement, string][] = [
  [PaintBoardIcon, "#ec6a8a"],
  [Video01Icon, "#8793ff"],
  [CodeIcon, "#30a46c"],
  [Globe02Icon, "#4459f9"],
  [Calculator01Icon, "#f7a600"],
  [Settings01Icon, "#6b6b6b"],
];

function TeachVisual() {
  return (
    <div className="teach-visual" aria-hidden="true">
      <div className="teach-col">
        {apps.slice(0, 3).map(([icon, tone], index) => (
          <span key={index} style={{ color: tone }}>
            <Glyph icon={icon} size={22} />
          </span>
        ))}
      </div>
      <div className="teach-centre">
        <AstridCursor x={0} y={0} label="step 2 of 5" />
      </div>
      <div className="teach-col">
        {apps.slice(3).map(([icon, tone], index) => (
          <span key={index} style={{ color: tone }}>
            <Glyph icon={icon} size={22} />
          </span>
        ))}
      </div>
    </div>
  );
}

function CalendarVisual() {
  return (
    <div className="cal-visual" aria-hidden="true">
      <div className="cal-head">
        <b>Thu 14</b>
        <small>November</small>
      </div>
      <div className="cal-grid">
        {["1 PM", "2 PM", "3 PM", "4 PM"].map((hour) => (
          <span key={hour}>{hour}</span>
        ))}
        <i className="cal-event existing">Design review</i>
        <i className="cal-event new">Dentist · 3:00</i>
      </div>
    </div>
  );
}

/** Clicky-style bento of everyday personal tasks. */
export function Everyday() {
  return (
    <div className="everyday">
      <Card area="mail" centered title="Search your mail" sub="Ask in plain words. Read-only, and only when you ask.">
        <MailVisual />
      </Card>
      <Card area="timer" title="Timers, alarms and reminders" sub="Say it once. Astrid keeps time, even after the chat is gone.">
        <TimerVisual />
      </Card>
      <Card area="eyes" title="It looks at you, not your screen" sub="Those eyes follow your cursor. It only sees the screen when you ask.">
        <EyesVisual />
      </Card>
      <Card area="teach" title="Teach you any app, step by step" sub="Design tools, editors, settings: guided as you go.">
        <TeachVisual />
      </Card>
      <Card area="cal" centered title="Put it on your calendar" sub="“Dentist Thursday at three.” Done, and shown to you first.">
        <CalendarVisual />
      </Card>
    </div>
  );
}
