//! ADR 0036 live canvas: the model describes a drawing as a fenced ```canvas
//! block of JSON lines, one shape per line. Shapes are validated here, as they
//! stream in, and only the cleaned shape reaches the presence UI, which draws
//! it with trusted templates. The model never supplies code or markup.
use serde_json::{Map, Value, json};

/// The virtual canvas the model draws on; the UI scales it to the screen.
pub const WIDTH: f64 = 1000.0;
pub const HEIGHT: f64 = 640.0;
const MAX_SHAPES: usize = 160;
const MAX_LINE: usize = 8 * 1024;
const MAX_POINTS: usize = 64;
const MAX_PATH: usize = 4000;
const MAX_TEXT: usize = 160;
const FENCE: &str = "```canvas";

pub const CANVAS_INSTRUCTIONS: &str = "When the user asks you to draw, design, sketch, diagram, illustrate, lay out or make anything visual, draw it on Astrid's on-screen canvas: put one fenced code block tagged `canvas` in your reply, with one JSON object per line and nothing else inside. The canvas is 1000 wide and 640 tall, origin top-left. Shapes (all numbers in canvas units): {\"shape\":\"title\",\"text\"}; {\"shape\":\"rect\",\"x\",\"y\",\"w\",\"h\",\"r\"?}; {\"shape\":\"ellipse\",\"cx\",\"cy\",\"rx\",\"ry\"}; {\"shape\":\"line\",\"points\":[[x,y],...]}; {\"shape\":\"polygon\",\"points\":[[x,y],...]}; {\"shape\":\"path\",\"d\": SVG path data using only M L H V C S Q T A Z and numbers}; {\"shape\":\"arrow\",\"from\":[x,y],\"to\":[x,y],\"bend\"?: -1..1}; {\"shape\":\"text\",\"x\",\"y\",\"text\",\"size\"?: 10..64,\"align\"?: \"start\"|\"middle\"|\"end\"}; {\"shape\":\"box\",\"x\",\"y\",\"w\",\"d\",\"h\"} for an isometric block whose front-bottom corner is at x,y; {\"shape\":\"point\",\"x\",\"y\",\"say\"} to move your cursor to something you drew and label it. Optional on any shape: \"id\", \"fill\", \"stroke\" (#hex or one of ink, paper, blue, violet, pink, red, orange, yellow, green, teal, grey, none), \"width\" 0.5..12, \"dashed\", \"sketchy\" (hand-drawn look), \"label\" (a short caption). Draw in a sensible order: big forms first, then details, labels and arrows, then 1–4 `point` lines walking the user through it. Use up to 120 shapes. Outside the block, say in one or two sentences what you drew; skip the brief for drawings.";

const COLORS: [&str; 12] = [
    "ink", "paper", "blue", "violet", "pink", "red", "orange", "yellow", "green", "teal", "grey",
    "none",
];

/// Finds the canvas block in streamed text and yields validated shapes.
#[derive(Default)]
pub struct CanvasStream {
    pending: String,
    state: State,
    shapes: usize,
}

#[derive(Default, PartialEq)]
enum State {
    #[default]
    Outside,
    Inside,
    Done,
}

impl CanvasStream {
    /// Feeds more model text; returns presence events to emit, in order.
    pub fn feed(&mut self, text: &str) -> Vec<Value> {
        let mut events = Vec::new();
        if self.state == State::Done {
            return events;
        }
        self.pending.push_str(text);
        loop {
            match self.state {
                State::Outside => {
                    let Some(start) = self.pending.find(FENCE) else {
                        // Keep a tail long enough to hold a split fence.
                        let keep = self.pending.len().saturating_sub(FENCE.len());
                        let cut = (keep..=self.pending.len())
                            .find(|i| self.pending.is_char_boundary(*i))
                            .unwrap_or(0);
                        self.pending.drain(..cut);
                        return events;
                    };
                    let Some(newline) = self.pending[start..].find('\n') else {
                        return events;
                    };
                    self.pending.drain(..start + newline + 1);
                    self.state = State::Inside;
                    events.push(
                        json!({"type":"canvas","event":"begin","width":WIDTH,"height":HEIGHT}),
                    );
                }
                State::Inside => {
                    let Some(newline) = self.pending.find('\n') else {
                        if self.pending.len() > MAX_LINE {
                            self.pending.clear();
                        }
                        return events;
                    };
                    let line: String = self.pending.drain(..=newline).collect();
                    self.line(line.trim(), &mut events);
                }
                State::Done => return events,
            }
        }
    }

    /// The reply ended: flush a last unterminated line and close the canvas.
    pub fn finish(&mut self) -> Vec<Value> {
        let mut events = Vec::new();
        if self.state == State::Inside {
            let line = std::mem::take(&mut self.pending);
            self.line(line.trim(), &mut events);
            if self.state == State::Inside {
                self.state = State::Done;
                events.push(json!({"type":"canvas","event":"end"}));
            }
        }
        events
    }

    pub fn started(&self) -> bool {
        self.state != State::Outside
    }

    fn line(&mut self, line: &str, events: &mut Vec<Value>) {
        if line.starts_with("```") {
            self.state = State::Done;
            events.push(json!({"type":"canvas","event":"end"}));
            return;
        }
        if line.is_empty() || line.len() > MAX_LINE || self.shapes >= MAX_SHAPES {
            return;
        }
        let Ok(Value::Object(raw)) = serde_json::from_str::<Value>(line) else {
            return;
        };
        if let Some(shape) = validate(&raw) {
            self.shapes += 1;
            events.push(json!({"type":"canvas","event":"shape","shape":shape}));
        }
    }
}

/// Removes the canvas block from the reply text shown and spoken.
pub fn strip(text: &str) -> String {
    let Some(start) = text.find(FENCE) else {
        return text.to_owned();
    };
    let after = &text[start + FENCE.len()..];
    let end = after
        .find("\n```")
        .map(|i| {
            let rest = &after[i + 4..];
            start + FENCE.len() + i + 4 + rest.find('\n').unwrap_or(rest.len())
        })
        .unwrap_or(text.len());
    format!("{}\n{}", text[..start].trim_end(), text[end..].trim_start())
        .trim()
        .to_owned()
}

fn num(raw: &Map<String, Value>, key: &str, min: f64, max: f64) -> Option<f64> {
    let value = raw.get(key)?.as_f64()?;
    value
        .is_finite()
        .then(|| (value.clamp(min, max) * 10.0).round() / 10.0)
}
fn x(raw: &Map<String, Value>, key: &str) -> Option<f64> {
    num(raw, key, -WIDTH * 0.25, WIDTH * 1.25)
}
fn y(raw: &Map<String, Value>, key: &str) -> Option<f64> {
    num(raw, key, -HEIGHT * 0.25, HEIGHT * 1.25)
}
fn size(raw: &Map<String, Value>, key: &str) -> Option<f64> {
    num(raw, key, 0.0, WIDTH * 1.5)
}
fn point(value: &Value) -> Option<Value> {
    let pair = value.as_array()?;
    let (px, py) = (pair.first()?.as_f64()?, pair.get(1)?.as_f64()?);
    (pair.len() == 2 && px.is_finite() && py.is_finite()).then(|| {
        json!([
            px.clamp(-WIDTH * 0.25, WIDTH * 1.25).round(),
            py.clamp(-HEIGHT * 0.25, HEIGHT * 1.25).round()
        ])
    })
}
fn points(raw: &Map<String, Value>, min: usize) -> Option<Value> {
    let list = raw.get("points")?.as_array()?;
    if list.len() < min || list.len() > MAX_POINTS {
        return None;
    }
    list.iter()
        .map(point)
        .collect::<Option<Vec<_>>>()
        .map(Value::from)
}
fn text(raw: &Map<String, Value>, key: &str) -> Option<String> {
    let value: String = raw
        .get(key)?
        .as_str()?
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_TEXT)
        .collect();
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}
fn color(value: &Value) -> Option<String> {
    let value = value.as_str()?.trim().to_ascii_lowercase();
    let hex = value
        .strip_prefix('#')
        .is_some_and(|h| matches!(h.len(), 3 | 6 | 8) && h.chars().all(|c| c.is_ascii_hexdigit()));
    (hex || COLORS.contains(&value.as_str())).then_some(value)
}
/// SVG path data: commands and numbers only, so nothing else can be smuggled in.
fn path_data(raw: &Map<String, Value>) -> Option<String> {
    let d = raw.get("d")?.as_str()?.trim();
    let ok = !d.is_empty()
        && d.len() <= MAX_PATH
        && d.starts_with(['M', 'm'])
        && d.chars().all(|c| {
            "MLHVCSQTAZmlhvcsqtaz".contains(c) || c.is_ascii_digit() || " ,.-+eE\n".contains(c)
        });
    ok.then(|| d.to_owned())
}

/// Returns a clean shape with only known fields, or None if it is unusable.
pub fn validate(raw: &Map<String, Value>) -> Option<Value> {
    let kind = raw.get("shape")?.as_str()?;
    let mut shape = Map::new();
    shape.insert("shape".into(), kind.into());
    let mut put = |key: &str, value: Option<Value>| -> Option<()> {
        shape.insert(key.into(), value?);
        Some(())
    };
    let f = |v: Option<f64>| v.map(Value::from);
    match kind {
        "title" => put("text", text(raw, "text").map(Value::from))?,
        "rect" => {
            put("x", f(x(raw, "x")))?;
            put("y", f(y(raw, "y")))?;
            put("w", f(size(raw, "w")))?;
            put("h", f(size(raw, "h")))?;
            let _ = put("r", f(num(raw, "r", 0.0, 200.0)));
        }
        "ellipse" => {
            put("cx", f(x(raw, "cx")))?;
            put("cy", f(y(raw, "cy")))?;
            put("rx", f(size(raw, "rx")))?;
            put("ry", f(size(raw, "ry")))?;
        }
        "line" => put("points", points(raw, 2))?,
        "polygon" => put("points", points(raw, 3))?,
        "path" => put("d", path_data(raw).map(Value::from))?,
        "arrow" => {
            put("from", raw.get("from").and_then(point))?;
            put("to", raw.get("to").and_then(point))?;
            let _ = put("bend", f(num(raw, "bend", -1.0, 1.0)));
        }
        "text" => {
            put("x", f(x(raw, "x")))?;
            put("y", f(y(raw, "y")))?;
            put("text", text(raw, "text").map(Value::from))?;
            let _ = put("size", f(num(raw, "size", 10.0, 64.0)));
            let align = raw.get("align").and_then(Value::as_str);
            let _ = put(
                "align",
                align
                    .filter(|a| ["start", "middle", "end"].contains(a))
                    .map(Value::from),
            );
        }
        "box" => {
            put("x", f(x(raw, "x")))?;
            put("y", f(y(raw, "y")))?;
            put("w", f(num(raw, "w", 1.0, 600.0)))?;
            put("d", f(num(raw, "d", 1.0, 600.0)))?;
            put("h", f(num(raw, "h", 0.0, 600.0)))?;
        }
        "point" => {
            put("x", f(x(raw, "x")))?;
            put("y", f(y(raw, "y")))?;
            let _ = put("say", text(raw, "say").map(Value::from));
        }
        _ => return None,
    }
    if let Some(id) = raw.get("id").and_then(Value::as_str) {
        let id: String = id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || "-_".contains(*c))
            .take(32)
            .collect();
        let _ = put("id", (!id.is_empty()).then(|| id.into()));
    }
    for key in ["fill", "stroke"] {
        let _ = put(key, raw.get(key).and_then(color).map(Value::from));
    }
    let _ = put("width", f(num(raw, "width", 0.5, 12.0)));
    for key in ["dashed", "sketchy"] {
        let _ = put(key, raw.get(key).and_then(Value::as_bool).map(Value::from));
    }
    if kind != "title" && kind != "text" && kind != "point" {
        let _ = put("label", text(raw, "label").map(Value::from));
    }
    Some(Value::Object(shape))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shapes(events: &[Value]) -> Vec<&Value> {
        events
            .iter()
            .filter(|e| e["event"] == "shape")
            .map(|e| &e["shape"])
            .collect()
    }

    #[test]
    fn shapes_stream_out_as_lines_complete_even_with_split_fences() {
        let reply = "Here's a mug.\n```canvas\n{\"shape\":\"title\",\"text\":\"Mug\"}\n{\"shape\":\"rect\",\"x\":300,\"y\":200,\"w\":200,\"h\":240,\"fill\":\"blue\"}\n```\nDone.";
        let mut stream = CanvasStream::default();
        let mut events = Vec::new();
        for chunk in reply.as_bytes().chunks(5) {
            events.extend(stream.feed(std::str::from_utf8(chunk).unwrap()));
        }
        events.extend(stream.finish());
        assert_eq!(events.first().unwrap()["event"], "begin");
        assert_eq!(events.last().unwrap()["event"], "end");
        let shapes = shapes(&events);
        assert_eq!(shapes.len(), 2);
        assert_eq!(shapes[1]["fill"], "blue");
        assert_eq!(shapes[1]["w"], 200.0);
    }

    #[test]
    fn a_reply_cut_off_inside_the_block_still_ends_the_canvas() {
        let mut stream = CanvasStream::default();
        let mut events =
            stream.feed("```canvas\n{\"shape\":\"ellipse\",\"cx\":1,\"cy\":2,\"rx\":3,\"ry\":4}");
        assert!(stream.started() && shapes(&events).is_empty());
        events.extend(stream.finish());
        assert_eq!(shapes(&events).len(), 1);
        assert_eq!(events.last().unwrap()["event"], "end");
        assert!(
            stream
                .feed("```canvas\n{\"shape\":\"title\",\"text\":\"again\"}\n")
                .is_empty()
        );
    }

    #[test]
    fn unknown_fields_bad_colors_markup_and_odd_paths_never_reach_the_ui() {
        let raw = |s: &str| serde_json::from_str::<Map<String, Value>>(s).unwrap();
        let shape = validate(&raw(
            r##"{"shape":"rect","x":1e9,"y":-5,"w":10,"h":10,"fill":"url(javascript:x)","stroke":"#ABC","onclick":"x","label":"hi\u0007"}"##,
        ))
        .unwrap();
        assert_eq!(shape["x"], 1250.0);
        assert!(shape.get("fill").is_none() && shape.get("onclick").is_none());
        assert_eq!(shape["stroke"], "#abc");
        assert_eq!(shape["label"], "hi");
        assert!(validate(&raw(r#"{"shape":"path","d":"M0 0 L10 10\"/><script>"}"#)).is_none());
        assert!(validate(&raw(r#"{"shape":"path","d":"M0 0 C 1 2 3 4 5 6Z"}"#)).is_some());
        assert!(validate(&raw(r#"{"shape":"script"}"#)).is_none());
        assert!(validate(&raw(r#"{"shape":"rect","x":1}"#)).is_none());
        assert!(validate(&raw(r#"{"shape":"line","points":[[0,0]]}"#)).is_none());
        let text = validate(&raw(
            r#"{"shape":"text","x":1,"y":2,"text":"<b>x</b>","align":"left"}"#,
        ))
        .unwrap();
        // Text is rendered as text, never markup; an unknown align is dropped.
        assert_eq!(text["text"], "<b>x</b>");
        assert!(text.get("align").is_none());
    }

    #[test]
    fn shapes_are_capped_per_reply() {
        let mut stream = CanvasStream::default();
        let line = "{\"shape\":\"point\",\"x\":1,\"y\":1}\n";
        let events = stream.feed(&format!("```canvas\n{}", line.repeat(MAX_SHAPES + 20)));
        assert_eq!(shapes(&events).len(), MAX_SHAPES);
    }

    #[test]
    fn the_block_is_stripped_from_the_spoken_reply() {
        let reply =
            "I drew a mug.\n```canvas\n{\"shape\":\"title\",\"text\":\"Mug\"}\n```\nWant a saucer?";
        assert_eq!(strip(reply), "I drew a mug.\nWant a saucer?");
        assert_eq!(strip("No drawing."), "No drawing.");
        assert_eq!(strip("Cut ```canvas\n{\"shape\""), "Cut");
    }
}
