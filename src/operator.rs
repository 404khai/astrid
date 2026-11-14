//! V2-05 operator (ADR 0038): a guided walkthrough on this Mac.
//!
//! An operator session is an explicit step state machine: look → propose a
//! step → the user chooses "Do it" or "Show me" → look again → verify →
//! advance. A step is never advanced past until a newer frame verifies it.
//! Every look is one ADR 0027 look whose approval is the user's button press
//! on a step card that says Astrid will look. "Do it" performs exactly the one
//! bounded action named on the card, on this Mac, through the presence host.
use crate::{
    cancellation::Cancellation,
    computer::CURSOR_STYLES,
    model::{Message, ModelProvider, ToolOutcome},
    permissions::{PermissionAction, PermissionPolicy},
    runtime::{self, PermissionHandler, RunConfig, RunOutcome},
    tools::{PermissionRequest, Tools},
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::io;

/// Looks per operator session (ADR 0038; ADR 0027 still caps each run at 2).
pub const MAX_LOOKS: usize = 12;
/// Steps a walkthrough may declare.
pub const MAX_STEPS: u32 = 8;
/// Failed checks of one step before the session stops.
pub const MAX_ATTEMPTS: u32 = 2;
/// Characters "Do it" may type in one step.
pub const MAX_TYPE_CHARS: usize = 200;
const MAX_TEXT: usize = 300;
/// Keys "Do it" may press, without modifiers.
pub const KEYS: [&str; 8] = [
    "enter", "tab", "escape", "space", "up", "down", "left", "right",
];

/// The one bounded action "Do it" may perform for a step.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// One left click at the step's point.
    Click,
    /// Click the step's point, then type this text (no newlines).
    Type(String),
    /// One key press, no modifiers.
    Key(String),
    /// Scroll at the step's point.
    Scroll { down: bool, notches: u32 },
}

impl Action {
    /// What the card says "Do it" will do.
    pub fn describe(&self) -> String {
        match self {
            Self::Click => "click there".into(),
            Self::Type(text) => format!("click there and type “{text}”"),
            Self::Key(key) => format!("press {key}"),
            Self::Scroll { down, notches } => format!(
                "scroll {} {notches} notch{}",
                if *down { "down" } else { "up" },
                if *notches == 1 { "" } else { "es" }
            ),
        }
    }

    /// The presence host's `act` payload.
    pub fn wire(&self) -> Value {
        match self {
            Self::Click => json!({"kind":"click"}),
            Self::Type(text) => json!({"kind":"type","text":text}),
            Self::Key(key) => json!({"kind":"key","key":key}),
            Self::Scroll { down, notches } => {
                json!({"kind":"scroll","direction":if *down {"down"} else {"up"},"notches":notches})
            }
        }
    }
}

/// One proposed step, in display points of the frame it was planned on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub index: u32,
    pub total: u32,
    pub instruction: String,
    pub x: u32,
    pub y: u32,
    pub cursor: String,
    pub action: Option<Action>,
    /// What the screen shows once the step is done; the next look checks it.
    pub check: String,
    /// The on-screen element (from the look's Accessibility list) the step
    /// targets. When set, x/y are that element's centre, not the model's guess.
    pub element: Option<u32>,
    /// Ask before acting: the step sends, deletes, buys, posts or submits.
    pub confirm: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    Step(Step),
    Finish(String),
}

/// The model's answer for one frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Whether this frame shows the pending step done. None without one.
    pub verified: Option<bool>,
    pub reason: String,
    pub next: Next,
}

/// Size of the look frame a report refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
}

fn text(value: &Value, key: &str, max: usize) -> Result<String, String> {
    let text = value[key]
        .as_str()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| format!("{key} must be non-empty text"))?;
    if text.chars().count() > max || text.chars().any(char::is_control) {
        return Err(format!("{key} must be at most {max} characters, one line"));
    }
    Ok(text.to_owned())
}

fn action(value: &Value) -> Result<Option<Action>, String> {
    if value.is_null() {
        return Ok(None);
    }
    Ok(Some(match value["kind"].as_str() {
        Some("click") => Action::Click,
        Some("type") => {
            let typed = value["text"].as_str().unwrap_or("");
            if typed.is_empty()
                || typed.chars().count() > MAX_TYPE_CHARS
                || typed.chars().any(char::is_control)
            {
                return Err(format!(
                    "type text must be 1...{MAX_TYPE_CHARS} characters, one line"
                ));
            }
            Action::Type(typed.to_owned())
        }
        Some("key") => {
            let key = value["key"].as_str().unwrap_or("");
            if !KEYS.contains(&key) {
                return Err(format!("key must be one of {}", KEYS.join(", ")));
            }
            Action::Key(key.to_owned())
        }
        Some("scroll") => {
            let down = match value["direction"].as_str() {
                Some("down") => true,
                Some("up") => false,
                _ => return Err("scroll direction must be up or down".into()),
            };
            let notches = value["notches"]
                .as_u64()
                .filter(|n| (1..=10).contains(n))
                .ok_or("scroll notches must be 1...10")?;
            Action::Scroll {
                down,
                notches: notches as u32,
            }
        }
        _ => return Err("action kind must be click, type, key or scroll".into()),
    }))
}

/// Reads and validates the trailing ```operator JSON block against the frame.
pub fn parse_report(reply: &str, frame: Frame) -> Result<Report, String> {
    let start = reply
        .rfind("```operator")
        .ok_or("the reply has no ```operator block")?;
    let rest = &reply[start + "```operator".len()..];
    let end = rest
        .find("```")
        .ok_or("the ```operator block is not closed")?;
    let value: Value = serde_json::from_str(rest[..end].trim())
        .map_err(|_| "the ```operator block is not JSON")?;
    let verified = match &value["verified"] {
        Value::Null => None,
        Value::Bool(b) => Some(*b),
        _ => return Err("verified must be true, false or absent".into()),
    };
    let reason = match value["reason"].as_str() {
        Some(_) => text(&value, "reason", MAX_TEXT)?,
        None => String::new(),
    };
    let next = match (&value["step"], &value["finish"]) {
        (Value::Null, Value::String(_)) => Next::Finish(text(&value, "finish", MAX_TEXT)?),
        (step @ Value::Object(_), Value::Null) => {
            let index = step["index"].as_u64().unwrap_or(0);
            let total = step["total"].as_u64().unwrap_or(0);
            if !(1..=u64::from(MAX_STEPS)).contains(&total) || !(1..=total).contains(&index) {
                return Err(format!(
                    "step index/total must satisfy 1 <= index <= total <= {MAX_STEPS}"
                ));
            }
            let element = match &step["element"] {
                Value::Null => None,
                id => Some(
                    id.as_u64()
                        .filter(|id| *id < 100_000)
                        .ok_or("element must be an id from the elements list")?
                        as u32,
                ),
            };
            let (x, y) = match (step["x"].as_u64(), step["y"].as_u64()) {
                (Some(x), Some(y)) if x < u64::from(frame.width) && y < u64::from(frame.height) => {
                    (x as u32, y as u32)
                }
                // An element target is resolved to its centre afterwards.
                _ if element.is_some() => (0, 0),
                _ => {
                    return Err(format!(
                        "step needs an element id, or x/y integers inside the {}x{} frame",
                        frame.width, frame.height
                    ));
                }
            };
            let cursor = step["cursor"].as_str().unwrap_or("arrow_click");
            if !CURSOR_STYLES.contains(&cursor) {
                return Err(format!(
                    "cursor must be one of {}",
                    CURSOR_STYLES.join(", ")
                ));
            }
            Next::Step(Step {
                index: index as u32,
                total: total as u32,
                instruction: text(step, "instruction", MAX_TEXT)?,
                x,
                y,
                cursor: cursor.to_owned(),
                action: action(&step["action"])?,
                check: text(step, "check", MAX_TEXT)?,
                element,
                confirm: step["confirm"] == true,
            })
        }
        _ => return Err("give exactly one of step or finish".into()),
    };
    Ok(Report {
        verified,
        reason,
        next,
    })
}

/// What the session does after a report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Advance {
    /// A new step (first, or the one after a verified step).
    Show(Step),
    /// The pending step was not verified; try it again.
    Retry {
        step: Step,
        reason: String,
    },
    Finished(String),
    Stopped(String),
}

/// The walkthrough's explicit state.
#[derive(Debug, Default)]
pub struct Walkthrough {
    pub goal: String,
    pub pending: Option<Step>,
    pub attempts: u32,
    pub looks: usize,
    /// Instructions of verified steps, in order.
    pub done: Vec<String>,
}

impl Walkthrough {
    pub fn new(goal: &str) -> Self {
        Self {
            goal: goal.to_owned(),
            ..Self::default()
        }
    }

    /// Counts one look; refuses past the session cap.
    pub fn record_look(&mut self) -> Result<(), String> {
        if self.looks >= MAX_LOOKS {
            return Err(format!("I've used all {MAX_LOOKS} looks for this session."));
        }
        self.looks += 1;
        Ok(())
    }

    /// Applies a report. Never moves past a pending step without `verified: true`.
    pub fn accept(&mut self, report: Report) -> Advance {
        let advance = match (self.pending.take(), report.next) {
            (None, Next::Step(step)) if step.index == 1 => Advance::Show(step),
            (None, Next::Finish(summary)) => Advance::Finished(summary),
            (None, Next::Step(_)) => {
                Advance::Stopped("The walkthrough must start at step 1.".into())
            }
            (Some(pending), next) if report.verified != Some(true) => {
                self.attempts += 1;
                let reason = if report.reason.is_empty() {
                    "I can't see that step done yet.".to_owned()
                } else {
                    report.reason
                };
                match next {
                    _ if self.attempts > MAX_ATTEMPTS => Advance::Stopped(format!(
                        "Step {} still isn't done: {reason}",
                        pending.index
                    )),
                    Next::Step(step) if step.index == pending.index => {
                        Advance::Retry { step, reason }
                    }
                    // Moving on from an unverified step is refused outright: its
                    // old coordinates belong to an older frame.
                    _ => Advance::Stopped(format!(
                        "I couldn't confirm step {} ({reason}), so I stopped rather than skip it.",
                        pending.index
                    )),
                }
            }
            (Some(pending), next) => {
                self.done.push(pending.instruction.clone());
                self.attempts = 0;
                match next {
                    Next::Step(step) if step.index == pending.index + 1 => Advance::Show(step),
                    Next::Finish(summary) => Advance::Finished(summary),
                    Next::Step(step) => Advance::Stopped(format!(
                        "Step {} was done, but the next step was numbered {}.",
                        pending.index, step.index
                    )),
                }
            }
        };
        if let Advance::Show(step) | Advance::Retry { step, .. } = &advance {
            self.pending = Some(step.clone());
        }
        advance
    }

    /// The task for one look run: the goal, progress and the pending check.
    pub fn task(&self) -> String {
        let mut task = format!(
            "You are Astrid's operator, guiding the user through a task on their own Mac one step at a time.\nGoal: {}\n",
            self.goal
        );
        if !self.done.is_empty() {
            task.push_str("Steps already done and verified:\n");
            for (n, step) in self.done.iter().enumerate() {
                task.push_str(&format!("{}. {step}\n", n + 1));
            }
        }
        match &self.pending {
            Some(step) => task.push_str(&format!(
                "Pending step {} of {}: {}\nIt is done when: {}\nSet \"verified\": true only if this new screenshot clearly shows that. Otherwise set false, say why in \"reason\", and give step {} again (with fresh coordinates).\n",
                step.index, step.total, step.instruction, step.check, step.index
            )),
            None => task.push_str("No step is pending yet; plan step 1 (or finish if the goal is already met).\n"),
        }
        task.push_str(OPERATOR_INSTRUCTIONS);
        task
    }
}

/// The reply format for each look run.
pub const OPERATOR_INSTRUCTIONS: &str = "Call `look` exactly once to see the screen now. The look result may include `elements`: the frontmost window's controls as [id, role, label, x, y, w, h] in display points, read from macOS Accessibility. Do not call point_at, sketch or say: Astrid points and acts for you. Then reply with only one fenced code block tagged `operator` containing JSON: {\"verified\": true|false (only when a step is pending), \"reason\": short reason, and either \"step\": {\"index\": n, \"total\": planned steps (at most 8), \"instruction\": one short imperative sentence naming the target, e.g. \"Click Like under the video\", \"element\": the id of the target from `elements` (always prefer this when the target is in the list; it is exact), \"x\": int, \"y\": int (only when the target is not in `elements`: the exact target in this screenshot's pixels), \"cursor\": one of click, press, tap, pointer, text, grab, select, crosshair, focus, zoom_in, zoom_out, pen, not_allowed, arrow, arrow_click (what happens there: click to click, text to type), \"action\": {\"kind\":\"click\"} | {\"kind\":\"type\",\"text\":...} | {\"kind\":\"key\",\"key\":\"enter\"|\"tab\"|\"escape\"|\"space\"|\"up\"|\"down\"|\"left\"|\"right\"} | {\"kind\":\"scroll\",\"direction\":\"up\"|\"down\",\"notches\":1-10} | null (Astrid performs the action itself; null only when the user must do it, e.g. typing a password or code), \"confirm\": true only when the step sends a message, deletes something, buys or pays, posts publicly or submits a form (Astrid asks first), \"check\": what the screen will show once the step is done} or \"finish\": one sentence saying the goal is done. Never put passwords or other secrets in type actions.";

/// A look run's request budget: the default text budget, with room in the
/// byte ceiling for one full frame (a PNG of at most 1 MiB is about 1.4 MB as
/// base64). The 512 KiB default would refuse every real screen.
pub fn look_budget() -> crate::context::ContextBudget {
    let text = crate::context::ContextBudget::default();
    crate::context::ContextBudget {
        max_request_bytes: text.max_request_bytes + 1_600_000,
        ..text
    }
}

/// Approves exactly one ADR 0027 look per user press, and nothing else.
struct PressedLook {
    armed: bool,
}
#[async_trait]
impl PermissionHandler for PressedLook {
    async fn decide(&mut self, request: &PermissionRequest) -> io::Result<bool> {
        let look = serde_json::from_str::<Value>(&request.command)
            .is_ok_and(|v| v["capability"] == "local.look");
        Ok(std::mem::take(&mut self.armed) && look)
    }
}

/// One look run: an approved frame, then the model's report for it.
#[async_trait]
pub trait Eyes: Send {
    /// Called only after a user press that approved this look.
    async fn look(&mut self, task: String, cancel: &Cancellation) -> Result<Report, String>;
}

/// The real eyes: an ADR 0027 run on a local-look `Tools`.
pub struct RuntimeEyes<'a> {
    pub provider: &'a dyn ModelProvider,
    pub tools: &'a Tools,
    pub model: String,
}

#[async_trait]
impl Eyes for RuntimeEyes<'_> {
    async fn look(&mut self, task: String, cancel: &Cancellation) -> Result<Report, String> {
        let config = RunConfig {
            routing: None,
            observability: None,
            context_budget: Some(look_budget()),
            model: self.model.clone(),
            task,
            max_model_calls: 3,
            // Computer tools need a non-read-only policy; writes stay denied
            // and any shell ask is refused by `PressedLook`.
            permissions: PermissionPolicy {
                read: PermissionAction::Allow,
                write: PermissionAction::Deny,
                execute: PermissionAction::Ask,
            },
        };
        let mut approvals = PressedLook { armed: true };
        let result = runtime::run(
            self.provider,
            self.tools,
            &mut approvals,
            config,
            cancel.clone(),
            None,
        )
        .await
        .map_err(|e| e.to_string())?;
        match &result.outcome {
            RunOutcome::Completed => {}
            RunOutcome::Cancelled => return Err("cancelled".into()),
            // A failed look says why (permission, frame size...), not just "stopped".
            RunOutcome::Failed { code, message } => {
                return Err(look_failure(&result.session.messages)
                    .unwrap_or_else(|| format!("{code}: {message}")));
            }
            RunOutcome::ModelCallLimitReached { .. } => {
                return Err("the model did not answer within its step limit".into());
            }
        }
        let observed = result
            .session
            .messages
            .iter()
            .rev()
            .find_map(|m| match m {
                Message::Tool(t) if t.name == "look" => match &t.outcome {
                    ToolOutcome::Success { data } if data["observed"].is_object() => {
                        Some(data["observed"].clone())
                    }
                    _ => None,
                },
                _ => None,
            })
            .ok_or("Astrid didn't get a look at the screen")?;
        let frame = Frame {
            width: observed["width"].as_u64().ok_or("look had no size")? as u32,
            height: observed["height"].as_u64().ok_or("look had no size")? as u32,
        };
        // Claude sees a scaled frame and answers in its pixels; map back.
        let claude = self.model.starts_with(crate::claude_code::PREFIX);
        let view = if claude {
            let (width, height) = crate::claude_code::claude_view(frame.width, frame.height);
            Frame { width, height }
        } else {
            frame
        };
        if let Some(dir) = crate::claude_code::debug_dir() {
            let _ = std::fs::write(
                dir.join(format!("reply-{}.txt", walkthrough_stamp())),
                format!(
                    "view {}x{}, frame {}x{}, elements: {}\n\n{}",
                    view.width,
                    view.height,
                    frame.width,
                    frame.height,
                    observed["elements"].as_array().map_or(0, Vec::len),
                    result.final_text
                ),
            );
        }
        let mut report = parse_report(&result.final_text, view)?;
        if let Next::Step(step) = &mut report.next {
            resolve_target(step, &observed["elements"], frame, view)?;
        }
        Ok(report)
    }
}

fn walkthrough_stamp() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Puts a step's point in display points of the look frame: an element's
/// centre when it names one (exact, from Accessibility), else the model's
/// point mapped from the image it saw.
pub fn resolve_target(
    step: &mut Step,
    elements: &Value,
    frame: Frame,
    view: Frame,
) -> Result<(), String> {
    if let Some(id) = step.element {
        let element = elements
            .as_array()
            .and_then(|list| list.iter().find(|e| e[0].as_u64() == Some(u64::from(id))))
            .ok_or_else(|| format!("element {id} is not in this look's elements"))?;
        let n = |i: usize| element[i].as_u64().unwrap_or(0);
        step.x = (n(3) + n(5) / 2).min(u64::from(frame.width - 1)) as u32;
        step.y = (n(4) + n(6) / 2).min(u64::from(frame.height - 1)) as u32;
        return Ok(());
    }
    if view != frame {
        step.x = (u64::from(step.x) * u64::from(frame.width) / u64::from(view.width))
            .min(u64::from(frame.width - 1)) as u32;
        step.y = (u64::from(step.y) * u64::from(frame.height) / u64::from(view.height))
            .min(u64::from(frame.height - 1)) as u32;
    }
    Ok(())
}

/// Why the run's look failed, in words the user can act on.
fn look_failure(messages: &[Message]) -> Option<String> {
    let failure = messages.iter().rev().find_map(|m| match m {
        Message::Tool(t) if t.name == "look" => match &t.outcome {
            ToolOutcome::Success { data } if data["successful"] == false => {
                data["failure"].as_str().map(str::to_owned)
            }
            ToolOutcome::Error { message, .. } => Some(message.clone()),
            _ => None,
        },
        _ => None,
    })?;
    Some(match failure.as_str() {
        "screen_recording_permission_required" => "Astrid Presence can't see the screen yet. In System Settings → Privacy & Security → Screen Recording, turn it on (if it's already on, remove it with − and add it again: a rebuilt app needs a fresh grant), then quit Astrid Presence from its menu-bar icon and open it again.".into(),
        "frame_png_exceeds_1mib" => "Your screen was too detailed to send within the 1 MiB frame limit, even simplified. Try again with fewer busy windows or a plainer wallpaper.".into(),
        "display_exceeds_frame_limits" => "That display is larger than 2560 points on a side, which looks don't support yet.".into(),
        "display_unavailable" => "That display isn't connected.".into(),
        "capture_timed_out" => "The screen capture didn't come back in time. Try again.".into(),
        other => format!("The look failed ({other})."),
    })
}

/// The buttons a step card can show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Look,
    DoIt,
    ShowMe,
    Done,
    Stop,
    Close,
}
impl Choice {
    pub fn id(self) -> &'static str {
        match self {
            Self::Look => "look",
            Self::DoIt => "do",
            Self::ShowMe => "show",
            Self::Done => "done",
            Self::Stop => "stop",
            Self::Close => "close",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Look => "Look",
            Self::DoIt => "Do it",
            Self::ShowMe => "Show me",
            Self::Done => "Done",
            Self::Stop => "Stop",
            Self::Close => "Close",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        [
            Self::Look,
            Self::DoIt,
            Self::ShowMe,
            Self::Done,
            Self::Stop,
            Self::Close,
        ]
        .into_iter()
        .find(|c| c.id() == id)
    }
}

/// Maps something the user said to one of the card's offered buttons, for
/// voice mode. Only offered buttons count; anything unclear is None and the
/// card keeps waiting. "Stop" words win, so "don't do it" never acts.
pub fn choice_from_speech(text: &str, offered: &[Choice]) -> Option<Choice> {
    let words: String = text
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let said = format!(
        " {} ",
        words.split_whitespace().collect::<Vec<_>>().join(" ")
    );
    let has = |phrases: &[&str]| phrases.iter().any(|p| said.contains(&format!(" {p} ")));
    let stop = [
        "stop",
        "cancel",
        "quit",
        "never mind",
        "nevermind",
        "no",
        "don't",
        "dont",
        "wait",
    ];
    let table: [(Choice, &[&str]); 6] = [
        // "I'll do it myself" means show me, before any "do it".
        (
            Choice::ShowMe,
            &["i'll do it", "i will do it", "myself", "let me"],
        ),
        (
            Choice::DoIt,
            &[
                "do it",
                "do that",
                "go ahead",
                "you do it",
                "for me",
                "please do",
                "click it",
                "type it",
            ],
        ),
        (
            Choice::ShowMe,
            &[
                "show me",
                "show",
                "how",
                "where",
                "i'll do it",
                "let me",
                "point",
            ],
        ),
        (
            Choice::Done,
            &[
                "done", "did it", "i did", "finished", "next", "check", "ready", "i'm done",
            ],
        ),
        (Choice::Look, &["look", "start", "begin", "go", "let's go"]),
        (
            Choice::Close,
            &["close", "thanks", "thank you", "got it", "bye", "great"],
        ),
    ];
    if has(&stop) {
        return [Choice::Stop, Choice::Close]
            .into_iter()
            .find(|c| offered.contains(c));
    }
    if let Some(choice) = table
        .iter()
        .find(|(choice, phrases)| offered.contains(choice) && has(phrases))
        .map(|(choice, _)| *choice)
    {
        return Some(choice);
    }
    // A plain yes picks the card's first button (Look, Do it, Done or Close).
    if has(&["yes", "yeah", "yep", "sure", "ok", "okay", "please"]) {
        return offered.first().copied().filter(|c| *c != Choice::Stop);
    }
    None
}

/// A step card: Astrid's eyes, "Step i of n", the text and its buttons,
/// placed next to a point of the display (or centred without one).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Card {
    pub step: Option<(u32, u32)>,
    pub text: String,
    pub buttons: Vec<Choice>,
    pub at: Option<(u32, u32)>,
}
impl Card {
    pub fn wire(&self, id: u64, display: u32) -> Value {
        json!({"type":"step","id":id,"display":display,
            "step":self.step.map(|s| s.0),"total":self.step.map(|s| s.1),
            "text":self.text,
            "buttons":self.buttons.iter().map(|b| json!({"id":b.id(),"label":b.label()})).collect::<Vec<_>>(),
            "x":self.at.map(|a| a.0),"y":self.at.map(|a| a.1)})
    }
}

/// What an operator session needs from the presence host and its user.
#[async_trait]
pub trait OperatorUi: Send + Sync {
    /// Shows a card and waits for one of its buttons (Stop on cancel).
    async fn card(&self, card: Card) -> Choice;
    /// One presence protocol v1 event (already validated against the frame).
    async fn present(&self, event: Value);
    /// "Do it": one bounded action at a point of the latest look frame. The
    /// host refuses it if the screen there changed since that look.
    async fn act(&self, step: &Step) -> Result<(), String>;
    /// Agent state for the ask panel.
    fn state(&self, state: &str, label: &str);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Completed(String),
    Stopped(String),
    Failed(String),
}

/// How the cards name the model: "Claude (sonnet)" for `claude-code/sonnet`.
pub fn model_label(model: &str) -> String {
    match model.strip_prefix(crate::claude_code::PREFIX) {
        Some(alias) => format!("Claude ({alias})"),
        None => model.to_owned(),
    }
}

/// Whether Astrid does the steps itself or shows the user how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// "Like this video": Astrid acts, then looks again to verify.
    Do,
    /// "Show me how to...": Astrid points and the user acts.
    Guide,
}

/// "Show me how...", "how do I...", "where is...": guide. Anything else: do it.
pub fn mode_for(goal: &str) -> Mode {
    let lower = goal.trim().to_lowercase();
    let guide = [
        "show me",
        "how do i",
        "how to",
        "how can i",
        "where is",
        "where's",
        "where do i",
        "teach me",
        "walk me through",
        "help me find",
        "guide me",
    ];
    if guide.iter().any(|p| lower.starts_with(p)) {
        Mode::Guide
    } else {
        Mode::Do
    }
}

/// Runs one walkthrough to its end. The user's ask starts it: there is no
/// card before the first look (the wave and Astrid's cursor show it's live).
pub async fn walk(
    goal: &str,
    mode: Mode,
    display: u32,
    eyes: &mut dyn Eyes,
    ui: &dyn OperatorUi,
    cancel: &Cancellation,
) -> Outcome {
    let mut walkthrough = Walkthrough::new(goal);
    ui.present(json!({"v":1,"type":"wave"})).await;
    let mut note = String::new();
    let mut looked_again = false;
    loop {
        if cancel.is_cancelled() {
            return Outcome::Stopped("Cancelled.".into());
        }
        if let Err(limit) = walkthrough.record_look() {
            return finish(ui, Outcome::Stopped(limit)).await;
        }
        ui.state("searching", "Looking");
        let report = match eyes.look(walkthrough.task(), cancel).await {
            Ok(report) => report,
            Err(error) if cancel.is_cancelled() => return Outcome::Stopped(error),
            Err(error) => return finish(ui, Outcome::Failed(error)).await,
        };
        let (step, retry) = match walkthrough.accept(report) {
            Advance::Show(step) => (step, None),
            Advance::Retry { step, reason } => (step, Some(reason)),
            Advance::Finished(summary) => return finish(ui, Outcome::Completed(summary)).await,
            Advance::Stopped(reason) => return finish(ui, Outcome::Stopped(reason)).await,
        };
        let point = |cursor: &str| json!({"v":1,"type":"point_at","x":step.x,"y":step.y,"display":display,"cursor":cursor});
        let at = Some((step.x, step.y));
        let counter = Some((step.index, step.total));
        ui.present(json!({"v":1,"type":"clear_ink"})).await;
        ui.state("working", &step.instruction);
        ui.present(point(&step.cursor)).await;

        if mode == Mode::Do && step.action.is_some() {
            if step.confirm {
                ui.state("listening", "Waiting for you");
                let ok = ui
                    .card(Card {
                        step: counter,
                        text: format!("{} Shall I?", step.instruction),
                        buttons: vec![Choice::DoIt, Choice::Stop],
                        at,
                    })
                    .await;
                if ok != Choice::DoIt {
                    return finish(ui, Outcome::Stopped("Stopped.".into())).await;
                }
                ui.state("working", &step.instruction);
            }
            match ui.act(&step).await {
                Ok(()) => {
                    looked_again = false;
                    continue;
                }
                // The screen moved under the target: look again once.
                Err(reason) if reason.contains("changed") && !looked_again => {
                    looked_again = true;
                    continue;
                }
                Err(reason) => note = format!("I couldn't do that myself: {reason}."),
            }
        }

        // Show the user: the step's cursor on the target, circled, until Done.
        ui.present(json!({"v":1,"type":"sketch","display":display,"shapes":[{"kind":"circle","x":step.x,"y":step.y,"r":26}]}))
            .await;
        ui.state("listening", "Waiting for you");
        let lead = match (retry, note.is_empty()) {
            (Some(reason), _) => format!("Not yet: {reason} "),
            (None, false) => format!("{note} "),
            _ => String::new(),
        };
        note.clear();
        let done = ui
            .card(Card {
                step: counter,
                text: format!("{lead}{} Tell me when you're done.", step.instruction),
                buttons: vec![Choice::Done, Choice::Stop],
                at,
            })
            .await;
        if done != Choice::Done {
            return finish(ui, Outcome::Stopped("Stopped.".into())).await;
        }
    }
}

async fn finish(ui: &dyn OperatorUi, outcome: Outcome) -> Outcome {
    ui.present(json!({"v":1,"type":"clear_ink"})).await;
    outcome
}

#[cfg(test)]
#[path = "operator_tests.rs"]
mod tests;
