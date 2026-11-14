//! V2-05 operator over `presence-ask`'s pipes (ADR 0038). The presence app is
//! the parent here, so the ADR 0027 host calls run the other way round: this
//! process asks for a `capture` on stdout and the app answers with `frame` on
//! stdin. Step cards, presentation events and "Do it" actions travel the same
//! private pipes; there is no socket, port or listener.
//!
//! stdout: {"type":"capture","request_id","display"} | {"type":"present","event":{v1}}
//!         | {"type":"step",..card} | {"type":"step_clear"}
//!         | {"type":"act","request_id","frame","display","x","y","action":{..}}
//! stdin:  {"type":"frame","request_id","png_base64"} | {"type":"capture_failed","request_id","reason"}
//!         | {"type":"choice","id","choice"} | {"type":"acted","request_id","outcome"}
use astrid::{
    cancellation::Cancellation,
    operator::{Card, Choice, OperatorUi, Step},
    presence::LookHost,
};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::sync::oneshot;

const MAX_PNG: usize = 1024 * 1024;
/// How long a step card waits for a press before the session stops.
const CARD_TIMEOUT: Duration = Duration::from_secs(600);
const ACT_TIMEOUT: Duration = Duration::from_secs(20);

type Waiters<K, T> = Mutex<HashMap<K, oneshot::Sender<T>>>;

/// The parent presence app, seen from `presence-ask`.
pub struct Bridge {
    emit: Box<dyn Fn(Value) + Send + Sync>,
    frames: Waiters<String, Result<Vec<u8>, String>>,
    choices: Waiters<u64, String>,
    acts: Waiters<String, String>,
    /// The card on screen and its buttons, for answers spoken in voice mode.
    current: Mutex<Option<(u64, Vec<Choice>)>>,
    /// Request id of the latest captured frame; "Do it" is checked against it.
    last_frame: Mutex<Option<String>>,
    /// Elements the app read with each frame, by request id, until taken.
    elements: Mutex<HashMap<String, Value>>,
    latest_elements: Mutex<Option<Value>>,
    next_card: AtomicU64,
}

fn take<K: std::hash::Hash + Eq, T>(
    waiters: &Waiters<K, T>,
    key: &K,
) -> Option<oneshot::Sender<T>> {
    waiters
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(key)
}

impl Bridge {
    pub fn new(emit: impl Fn(Value) + Send + Sync + 'static) -> Self {
        Self {
            emit: Box::new(emit),
            frames: Mutex::default(),
            choices: Mutex::default(),
            acts: Mutex::default(),
            current: Mutex::default(),
            last_frame: Mutex::default(),
            elements: Mutex::default(),
            latest_elements: Mutex::default(),
            next_card: AtomicU64::new(0),
        }
    }

    /// Routes one stdin message from the app. Returns false if it isn't ours.
    pub fn receive(&self, message: &Value) -> bool {
        let id = message["request_id"].as_str().unwrap_or("").to_owned();
        match message["type"].as_str() {
            Some("frame") => {
                let png = message["png_base64"]
                    .as_str()
                    .filter(|b| b.len() <= MAX_PNG.div_ceil(3) * 4)
                    .and_then(|b| STANDARD.decode(b).ok())
                    .filter(|png| png.len() <= MAX_PNG)
                    .ok_or_else(|| "host frame was malformed or too large".to_string());
                if let Some(tx) = take(&self.frames, &id) {
                    if message["elements"].is_array() {
                        self.elements
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .insert(id.clone(), message["elements"].clone());
                    }
                    let _ = tx.send(png);
                }
            }
            Some("capture_failed") => {
                let reason = message["reason"].as_str().unwrap_or("capture_failed");
                if let Some(tx) = take(&self.frames, &id) {
                    let _ = tx.send(Err(format!("host capture failed: {reason}")));
                }
            }
            Some("choice") => {
                let card = message["id"].as_u64().unwrap_or(0);
                if let Some(tx) = take(&self.choices, &card) {
                    let _ = tx.send(message["choice"].as_str().unwrap_or("stop").to_owned());
                }
            }
            Some("utterance") => {
                // Voice mode: "do it", "show me", "done", "stop"... for the card on screen.
                let said: String = message["text"]
                    .as_str()
                    .unwrap_or("")
                    .chars()
                    .take(500)
                    .collect();
                let current = self
                    .current
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                let choice = current
                    .as_ref()
                    .and_then(|(_, offered)| astrid::operator::choice_from_speech(&said, offered));
                (self.emit)(json!({"type":"heard","text":said,"choice":choice.map(Choice::id)}));
                if let (Some((id, _)), Some(choice)) = (current, choice)
                    && let Some(tx) = take(&self.choices, &id)
                {
                    let _ = tx.send(choice.id().to_owned());
                }
            }
            Some("acted") => {
                if let Some(tx) = take(&self.acts, &id) {
                    let _ = tx.send(message["outcome"].as_str().unwrap_or("failed").to_owned());
                }
            }
            _ => return false,
        }
        true
    }
}

#[async_trait]
impl LookHost for Bridge {
    async fn capture(&self, display: u32, cancel: &Cancellation) -> Result<Vec<u8>, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.frames
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone(), tx);
        (self.emit)(json!({"type":"capture","request_id":id,"display":display}));
        let result = tokio::select! {biased;
            _ = cancel.cancelled() => Err("capture cancelled".to_string()),
            _ = tokio::time::sleep(Duration::from_secs(10)) => Err("host capture timed out".to_string()),
            reply = rx => reply.unwrap_or_else(|_| Err("presence app went away".into())),
        };
        take(&self.frames, &id);
        // Only this frame's elements, never an older frame's.
        let elements = self
            .elements
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&id);
        *self
            .latest_elements
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = elements.filter(|_| result.is_ok());
        if result.is_ok() {
            *self.last_frame.lock().unwrap_or_else(|e| e.into_inner()) = Some(id);
        }
        result
    }
    async fn present(&self, event: Value) -> Result<(), String> {
        (self.emit)(json!({"type":"present","event":event}));
        Ok(())
    }
    fn elements(&self) -> Option<Value> {
        self.latest_elements
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
}

/// One operator session's view of the bridge.
pub struct Session {
    pub bridge: Arc<Bridge>,
    pub display: u32,
    pub cancel: Cancellation,
}

#[async_trait]
impl OperatorUi for Session {
    async fn card(&self, card: Card) -> Choice {
        let id = self.bridge.next_card.fetch_add(1, Ordering::Relaxed) + 1;
        let (tx, rx) = oneshot::channel();
        self.bridge
            .choices
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, tx);
        *self
            .bridge
            .current
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some((id, card.buttons.clone()));
        (self.bridge.emit)(card.wire(id, self.display));
        let choice = tokio::select! {biased;
            _ = self.cancel.cancelled() => None,
            _ = tokio::time::sleep(CARD_TIMEOUT) => None,
            reply = rx => reply.ok(),
        };
        take(&self.bridge.choices, &id);
        self.bridge
            .current
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take_if(|(shown, _)| *shown == id);
        (self.bridge.emit)(json!({"type":"step_clear","id":id}));
        // Only a button the card actually offered counts.
        choice
            .and_then(|c| Choice::from_id(&c))
            .filter(|c| card.buttons.contains(c))
            .unwrap_or(Choice::Stop)
    }
    async fn present(&self, event: Value) {
        (self.bridge.emit)(json!({"type":"present","event":event}));
    }
    async fn act(&self, step: &Step) -> Result<(), String> {
        let action = step.action.as_ref().ok_or("this step has no action")?;
        let frame = self
            .bridge
            .last_frame
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or("no look to check the screen against")?;
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        self.bridge
            .acts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone(), tx);
        (self.bridge.emit)(
            json!({"type":"act","request_id":id,"frame":frame,"display":self.display,
            "x":step.x,"y":step.y,"action":action.wire()}),
        );
        let outcome = tokio::select! {biased;
            _ = self.cancel.cancelled() => None,
            _ = tokio::time::sleep(ACT_TIMEOUT) => None,
            reply = rx => reply.ok(),
        };
        take(&self.bridge.acts, &id);
        match outcome.as_deref() {
            Some("done") => Ok(()),
            Some("stale") => Err("the screen changed since I looked".into()),
            Some("accessibility_permission_required") => Err(
                "to click for you I need Accessibility: System Settings → Privacy & Security → Accessibility → Astrid Presence".into(),
            ),
            Some(other) => Err(format!("the host refused: {other}")),
            None => Err("the host didn't answer".into()),
        }
    }
    fn state(&self, state: &str, label: &str) {
        (self.bridge.emit)(json!({"type":"state","state":state,"label":label}));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge() -> (Arc<Bridge>, Arc<Mutex<Vec<Value>>>) {
        let out: Arc<Mutex<Vec<Value>>> = Arc::default();
        let sink = out.clone();
        (
            Arc::new(Bridge::new(move |v| sink.lock().unwrap().push(v))),
            out,
        )
    }
    fn last(out: &Arc<Mutex<Vec<Value>>>, kind: &str) -> Value {
        out.lock()
            .unwrap()
            .iter()
            .rev()
            .find(|v| v["type"] == kind)
            .cloned()
            .unwrap()
    }

    #[tokio::test]
    async fn captures_go_out_and_frames_come_back_by_request_id() {
        let (bridge, out) = bridge();
        let task = {
            let bridge = bridge.clone();
            tokio::spawn(async move { bridge.capture(0, &Cancellation::default()).await })
        };
        tokio::task::yield_now().await;
        let request = last(&out, "capture");
        assert_eq!(request["display"], 0);
        // A frame for another request is ignored; the right one is delivered.
        assert!(bridge.receive(&json!({"type":"frame","request_id":"other","png_base64":"AAAA"})));
        bridge.receive(&json!({"type":"frame","request_id":request["request_id"],"png_base64":STANDARD.encode(b"png")}));
        assert_eq!(task.await.unwrap().unwrap(), b"png");
        assert!(!bridge.receive(&json!({"type":"ask"})));
    }

    #[tokio::test]
    async fn spoken_answers_press_the_card_on_screen() {
        let (bridge, out) = bridge();
        let session = Arc::new(Session {
            bridge: bridge.clone(),
            display: 0,
            cancel: Cancellation::default(),
        });
        let card = Card {
            step: Some((2, 3)),
            text: "Click Export.".into(),
            buttons: vec![Choice::DoIt, Choice::ShowMe, Choice::Stop],
            at: Some((10, 20)),
        };
        let task = {
            let session = session.clone();
            tokio::spawn(async move { session.card(card).await })
        };
        tokio::task::yield_now().await;
        // Unclear speech presses nothing; the card keeps waiting.
        bridge.receive(&json!({"type":"utterance","text":"hmm what's that"}));
        assert_eq!(last(&out, "heard")["choice"], Value::Null);
        assert!(!task.is_finished());
        bridge.receive(&json!({"type":"utterance","text":"Yeah, go ahead and do it."}));
        assert_eq!(task.await.unwrap(), Choice::DoIt);
        assert_eq!(last(&out, "heard")["choice"], "do");
        // With no card on screen, speech is heard but presses nothing.
        bridge.receive(&json!({"type":"utterance","text":"stop"}));
        assert_eq!(last(&out, "heard")["choice"], Value::Null);
    }

    #[tokio::test]
    async fn cards_accept_only_offered_buttons_and_acts_need_a_look() {
        let (bridge, out) = bridge();
        let session = Arc::new(Session {
            bridge: bridge.clone(),
            display: 0,
            cancel: Cancellation::default(),
        });
        let card = Card {
            step: Some((1, 2)),
            text: "Click Export.".into(),
            buttons: vec![Choice::ShowMe, Choice::Stop],
            at: Some((10, 20)),
        };
        let pick = |choice: &'static str| {
            let session = session.clone();
            let card = card.clone();
            let bridge = bridge.clone();
            let out = out.clone();
            async move {
                let task = tokio::spawn(async move { session.card(card).await });
                tokio::task::yield_now().await;
                let shown = last(&out, "step");
                bridge.receive(&json!({"type":"choice","id":shown["id"],"choice":choice}));
                task.await.unwrap()
            }
        };
        assert_eq!(pick("show").await, Choice::ShowMe);
        // "Do it" wasn't offered on this card, so it counts as Stop.
        assert_eq!(pick("do").await, Choice::Stop);
        let shown = last(&out, "step");
        assert_eq!(shown["step"], 1);
        assert_eq!(shown["buttons"][0]["label"], "Show me");

        let step = Step {
            index: 1,
            total: 1,
            instruction: "Click.".into(),
            x: 1,
            y: 2,
            cursor: "click".into(),
            action: Some(astrid::operator::Action::Click),
            check: "x".into(),
            element: None,
            confirm: false,
        };
        assert!(session.act(&step).await.is_err(), "no look yet");
        *bridge.last_frame.lock().unwrap() = Some("frame-1".into());
        let task = {
            let session = session.clone();
            let step = step.clone();
            tokio::spawn(async move { session.act(&step).await })
        };
        tokio::task::yield_now().await;
        let act = last(&out, "act");
        assert_eq!(act["frame"], "frame-1");
        assert_eq!(act["action"], json!({"kind":"click"}));
        bridge.receive(&json!({"type":"acted","request_id":act["request_id"],"outcome":"stale"}));
        assert_eq!(
            task.await.unwrap().unwrap_err(),
            "the screen changed since I looked"
        );
    }
}
