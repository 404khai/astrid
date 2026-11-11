use super::*;
use serde_json::json;
use std::sync::{Arc, Mutex, atomic::AtomicUsize};

fn owner() -> Owner {
    Owner {
        principal: "principal-sentinel".into(),
        run_id: RunId::default(),
        machine_id: format!("m_{}", "a".repeat(26)),
    }
}
fn controller() -> ActionController {
    ActionController::new(owner()).unwrap()
}
fn observation(c: &mut ActionController, cancel: &Cancellation) -> FrameId {
    c.record_observation(
        ObservationMetadata {
            owner: c.owner.clone(),
            width: 1280,
            height: 800,
            admitted_at: Instant::now(),
        },
        cancel,
    )
    .unwrap()
}
fn click(frame_id: FrameId) -> Arguments {
    Arguments {
        frame_id,
        action: Action::Click {
            x: 1279,
            y: 799,
            button: Button::Left,
        },
    }
}
fn request() -> ApprovalRequest {
    ApprovalRequest {
        owner: owner(),
        tool_call_id: ToolCallId::default(),
        provider_call_id: "provider-call-sentinel".into(),
        args: click(FrameId(uuid::Uuid::new_v4())),
        width: 1280,
        height: 800,
        policy: POLICY,
    }
}
struct Yes;
#[async_trait]
impl ActionApproval for Yes {
    async fn approve(&self, r: &ApprovalRequest) -> Option<ActionGrant> {
        Some(ActionGrant::approve(r.clone(), RUN_WINDOW).unwrap())
    }
}
struct No;
#[async_trait]
impl ActionApproval for No {
    async fn approve(&self, _: &ApprovalRequest) -> Option<ActionGrant> {
        None
    }
}
#[derive(Default)]
struct Ack(Mutex<Vec<ApprovalRequest>>);
#[async_trait]
impl ActionTransport for Ack {
    async fn act(&self, r: &ApprovalRequest) -> Result<(), TransportFailure> {
        self.0.lock().unwrap().push(r.clone());
        Ok(())
    }
}
async fn perform(
    c: &mut ActionController,
    args: Arguments,
    broker: Option<&dyn ActionApproval>,
    t: &dyn ActionTransport,
    cancel: &Cancellation,
) -> ActionReport {
    c.perform(
        ToolCallId::default(),
        "provider-call-sentinel".into(),
        args,
        broker,
        t,
        cancel,
    )
    .await
}

#[tokio::test(start_paused = true)]
async fn four_actions_are_exactly_bound_and_ack_is_only_input_acknowledgement() {
    let actions = [
        Action::Click {
            x: 0,
            y: 0,
            button: Button::Middle,
        },
        Action::Type {
            text: "  café\n世界 🦀\t  ".into(),
        },
        Action::Scroll {
            direction: Direction::Down,
            notches: 10,
        },
        Action::Key {
            key: "Enter".into(),
            modifiers: vec![Modifier::Ctrl, Modifier::Shift],
        },
    ];
    for action in actions {
        let mut c = controller();
        let cancel = Cancellation::default();
        let t = Ack::default();
        let frame = observation(&mut c, &cancel);
        let args = Arguments {
            frame_id: frame,
            action: action.clone(),
        };
        assert_eq!(
            perform(&mut c, args.clone(), Some(&Yes), &t, &cancel).await,
            ActionReport::InputAcknowledged
        );
        {
            let requests = t.0.lock().unwrap();
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].args, args);
            assert_eq!(requests[0].owner, c.owner);
            assert_eq!(requests[0].width, 1280);
            assert_eq!(requests[0].height, 800);
        }
        assert_eq!(c.accounting().admitted_actions, 1);
        assert!(!c.accounting().outcome_unknown);
        assert_eq!(c.accounting().typed_bytes, action.text_bytes());
        assert_eq!(
            perform(&mut c, click(frame), Some(&Yes), &t, &cancel).await,
            ActionReport::NotDispatched(Rejection::StaleFrame)
        );
    }
}

#[tokio::test(start_paused = true)]
async fn no_broker_and_denial_never_dispatch_or_spend_action_budget() {
    for broker in [None, Some(&No as &dyn ActionApproval)] {
        let mut c = controller();
        let cancel = Cancellation::default();
        let t = Ack::default();
        let f = observation(&mut c, &cancel);
        assert_eq!(
            perform(&mut c, click(f), broker, &t, &cancel).await,
            ActionReport::NotDispatched(Rejection::Denied)
        );
        assert!(t.0.lock().unwrap().is_empty());
        assert_eq!(c.accounting().admitted_actions, 0);
        assert!(c.frame.is_none());
    }
}

#[test]
fn action_schema_rejects_unknown_fields_sequences_and_invalid_values() {
    let f = FrameId(uuid::Uuid::new_v4());
    let bad = [
        json!({"kind":"click","x":0,"y":0,"button":"left","double":true}),
        json!({"kind":"click","x":-1,"y":0,"button":"left"}),
        json!({"kind":"click","x":true,"y":0,"button":"left"}),
        json!({"kind":"key","key":"Enter","modifiers":[],"command":"injected"}),
        json!({"kind":"shell","command":"injected"}),
        json!({"kind":"move","x":1,"y":1}),
        json!({"kind":"type","text":"ok","delay":1}),
        json!([{"kind":"key","key":"Enter"}]),
    ];
    for value in bad {
        assert!(serde_json::from_value::<Arguments>(json!({"frame_id":f,"action":value})).is_err());
    }
    assert!(
        serde_json::from_value::<Arguments>(
            json!({"frame_id":f,"action":{"kind":"type","text":"x"},"target":"elsewhere"})
        )
        .is_err()
    );
    for action in [
        Action::Click {
            x: 1280,
            y: 0,
            button: Button::Left,
        },
        Action::Click {
            x: 0,
            y: 800,
            button: Button::Right,
        },
        Action::Type { text: "".into() },
        Action::Type {
            text: "x\0y".into(),
        },
        Action::Type {
            text: "é".repeat(513),
        },
        Action::Scroll {
            direction: Direction::Left,
            notches: 0,
        },
        Action::Scroll {
            direction: Direction::Right,
            notches: 11,
        },
        Action::Key {
            key: "Return".into(),
            modifiers: vec![],
        },
        Action::Key {
            key: "a+b".into(),
            modifiers: vec![],
        },
        Action::Key {
            key: "a".into(),
            modifiers: vec![Modifier::Ctrl, Modifier::Ctrl],
        },
    ] {
        assert_eq!(action.validate(1280, 800), Err(Rejection::Arguments));
    }
}

#[test]
fn every_native_button_direction_and_allowlisted_key_is_supported() {
    for button in [Button::Left, Button::Right, Button::Middle] {
        assert!(Action::Click { x: 0, y: 0, button }.validate(1, 1).is_ok());
    }
    for direction in [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ] {
        assert!(
            Action::Scroll {
                direction,
                notches: 1
            }
            .validate(1, 1)
            .is_ok()
        );
    }
    let names = [
        "Enter",
        "Escape",
        "Tab",
        "Backspace",
        "Delete",
        "Left",
        "Right",
        "Up",
        "Down",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "Space",
    ];
    for key in names
        .iter()
        .map(|s| s.to_string())
        .chain(('a'..='z').chain('0'..='9').map(|c| c.to_string()))
    {
        assert!(
            Action::Key {
                key,
                modifiers: vec![Modifier::Ctrl, Modifier::Alt, Modifier::Shift]
            }
            .validate(1, 1)
            .is_ok()
        );
    }
}

#[test]
fn all_identity_frame_geometry_and_action_binding_dimensions_are_checked() {
    let r = request();
    let grant = ActionGrant::approve(r.clone(), RUN_WINDOW).unwrap();
    let cancel = Cancellation::default();
    let mut changes = Vec::new();
    let mut x = r.clone();
    x.owner.principal = "other".into();
    changes.push(x);
    let mut x = r.clone();
    x.owner.run_id = RunId::default();
    changes.push(x);
    let mut x = r.clone();
    x.owner.machine_id = format!("m_{}", "b".repeat(26));
    changes.push(x);
    let mut x = r.clone();
    x.tool_call_id = ToolCallId::default();
    changes.push(x);
    let mut x = r.clone();
    x.provider_call_id = "other".into();
    changes.push(x);
    let mut x = r.clone();
    x.args.frame_id = FrameId(uuid::Uuid::new_v4());
    changes.push(x);
    let mut x = r.clone();
    x.width = 1279;
    changes.push(x);
    let mut x = r.clone();
    x.height = 799;
    changes.push(x);
    let mut x = r.clone();
    x.policy = "different";
    changes.push(x);
    let mut x = r.clone();
    x.args.action = Action::Type {
        text: "other-sentinel".into(),
    };
    changes.push(x);
    for changed in changes {
        assert_eq!(grant.consume(&changed, &cancel), Err(Rejection::Binding));
    }
    assert_eq!(grant.consume(&r, &cancel), Ok(()));
    assert_eq!(grant.consume(&r, &cancel), Err(Rejection::Consumed));
}

#[test]
fn concurrent_grant_consumption_admits_exactly_once() {
    let r = request();
    let g = Arc::new(ActionGrant::approve(r.clone(), RUN_WINDOW).unwrap());
    let successes = Arc::new(AtomicUsize::new(0));
    std::thread::scope(|scope| {
        for _ in 0..32 {
            let g = g.clone();
            let r = &r;
            let successes = &successes;
            scope.spawn(move || {
                if g.consume(r, &Cancellation::default()).is_ok() {
                    successes.fetch_add(1, Ordering::SeqCst);
                }
            });
        }
    });
    assert_eq!(successes.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn grant_expiry_and_cancel_do_not_consume_before_admission() {
    let r = request();
    let cancel = Cancellation::default();
    for duration in [Duration::ZERO, Duration::from_secs(31)] {
        assert!(ActionGrant::approve(r.clone(), duration).is_err());
    }
    let g = ActionGrant::approve(r.clone(), Duration::from_secs(1)).unwrap();
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(g.consume(&r, &cancel), Err(Rejection::Expired));
    assert!(!g.used.load(Ordering::Acquire));
    let g = ActionGrant::approve(r.clone(), RUN_WINDOW).unwrap();
    cancel.cancel();
    assert_eq!(g.consume(&r, &cancel), Err(Rejection::Cancelled));
    assert!(!g.used.load(Ordering::Acquire));
}

#[tokio::test(start_paused = true)]
async fn frame_age_boundary_replacement_and_geometry_invalidation_are_fail_closed() {
    let cancel = Cancellation::default();
    let t = Ack::default();
    for age in [Duration::from_millis(4999), FRAME_AGE] {
        let mut c = controller();
        let f = observation(&mut c, &cancel);
        tokio::time::advance(age).await;
        let result = perform(&mut c, click(f), Some(&Yes), &t, &cancel).await;
        if age < FRAME_AGE {
            assert_eq!(result, ActionReport::InputAcknowledged);
        } else {
            assert_eq!(result, ActionReport::NotDispatched(Rejection::StaleFrame));
        }
    }
    let mut c = controller();
    let old = observation(&mut c, &cancel);
    let new = observation(&mut c, &cancel);
    assert_ne!(old, new);
    assert_eq!(
        perform(&mut c, click(old), Some(&Yes), &t, &cancel).await,
        ActionReport::NotDispatched(Rejection::StaleFrame)
    );
    assert!(c.frame.is_none());
    let mut c = controller();
    let f = observation(&mut c, &cancel);
    c.invalidate_geometry();
    assert_eq!(
        perform(&mut c, click(f), Some(&Yes), &t, &cancel).await,
        ActionReport::NotDispatched(Rejection::StaleFrame)
    );
}

#[tokio::test(start_paused = true)]
async fn observation_metadata_requires_owner_valid_native_geometry_and_nonfuture_time() {
    let mut c = controller();
    let cancel = Cancellation::default();
    for (width, height) in [
        (0, 1),
        (1, 0),
        (2561, 1),
        (2560, 2560),
        (u32::MAX, u32::MAX),
    ] {
        assert_eq!(
            c.record_observation(
                ObservationMetadata {
                    owner: c.owner.clone(),
                    width,
                    height,
                    admitted_at: Instant::now()
                },
                &cancel
            ),
            Err(Rejection::Geometry)
        );
    }
    for at in [
        Instant::now() + Duration::from_millis(1),
        Instant::now() - FRAME_AGE,
    ] {
        assert_eq!(
            c.record_observation(
                ObservationMetadata {
                    owner: c.owner.clone(),
                    width: 1280,
                    height: 800,
                    admitted_at: at
                },
                &cancel
            ),
            Err(Rejection::StaleFrame)
        );
    }
    for changed in [
        owner(),
        Owner {
            principal: "other".into(),
            ..c.owner.clone()
        },
        Owner {
            machine_id: format!("m_{}", "b".repeat(26)),
            ..c.owner.clone()
        },
    ] {
        assert_eq!(
            c.record_observation(
                ObservationMetadata {
                    owner: changed,
                    width: 1280,
                    height: 800,
                    admitted_at: Instant::now()
                },
                &cancel
            ),
            Err(Rejection::Binding)
        );
    }
    assert_eq!(c.accounting().installed_receipts, 0);
    assert!(
        c.record_observation(
            ObservationMetadata {
                owner: c.owner.clone(),
                width: 2560,
                height: 1600,
                admitted_at: Instant::now()
            },
            &cancel
        )
        .is_ok()
    );
}

struct Changed;
#[async_trait]
impl ActionApproval for Changed {
    async fn approve(&self, r: &ApprovalRequest) -> Option<ActionGrant> {
        let mut changed = r.clone();
        changed.args.action = Action::Key {
            key: "Enter".into(),
            modifiers: vec![],
        };
        Some(ActionGrant::approve(changed, RUN_WINDOW).unwrap())
    }
}
struct Delay(Duration);
#[async_trait]
impl ActionApproval for Delay {
    async fn approve(&self, r: &ApprovalRequest) -> Option<ActionGrant> {
        tokio::time::sleep(self.0).await;
        Yes.approve(r).await
    }
}
struct CancelOnApproval(Cancellation);
#[async_trait]
impl ActionApproval for CancelOnApproval {
    async fn approve(&self, r: &ApprovalRequest) -> Option<ActionGrant> {
        let grant = Yes.approve(r).await;
        self.0.cancel();
        grant
    }
}
struct Expired;
#[async_trait]
impl ActionApproval for Expired {
    async fn approve(&self, r: &ApprovalRequest) -> Option<ActionGrant> {
        let grant = ActionGrant::approve(r.clone(), Duration::from_secs(1)).unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        Some(grant)
    }
}

#[tokio::test(start_paused = true)]
async fn changed_expired_cancelled_and_late_approvals_never_dispatch() {
    let t = Ack::default();
    let cancel = Cancellation::default();
    let changed = Changed;
    let expired = Expired;
    let late = Delay(FRAME_AGE + Duration::from_secs(1));
    for (broker, expected) in [
        (&changed as &dyn ActionApproval, Rejection::Binding),
        (&expired, Rejection::Expired),
        (&late, Rejection::ApprovalDeadline),
    ] {
        let mut c = controller();
        let f = observation(&mut c, &cancel);
        assert_eq!(
            perform(&mut c, click(f), Some(broker), &t, &cancel).await,
            ActionReport::NotDispatched(expected)
        );
        assert_eq!(c.accounting().admitted_actions, 0);
        assert!(c.frame.is_none());
    }
    let mut c = controller();
    let f = observation(&mut c, &cancel);
    let broker = CancelOnApproval(cancel.clone());
    assert_eq!(
        perform(&mut c, click(f), Some(&broker), &t, &cancel).await,
        ActionReport::NotDispatched(Rejection::Cancelled)
    );
    assert!(t.0.lock().unwrap().is_empty());
}

#[tokio::test(start_paused = true)]
async fn approval_wait_cancellation_wins_and_frame_expiration_does_not_refresh() {
    let mut c = controller();
    let cancel = Cancellation::default();
    let t = Ack::default();
    let f = observation(&mut c, &cancel);
    let canceller = async {
        tokio::time::sleep(Duration::from_secs(1)).await;
        cancel.cancel();
    };
    let delayed_approval = Delay(Duration::from_secs(10));
    let (result, ()) = tokio::join!(
        perform(&mut c, click(f), Some(&delayed_approval), &t, &cancel),
        canceller
    );
    assert_eq!(result, ActionReport::NotDispatched(Rejection::Cancelled));
    assert!(c.accounting().closed);
    assert!(t.0.lock().unwrap().is_empty());
    let mut c = controller();
    let cancel = Cancellation::default();
    let f = observation(&mut c, &cancel);
    tokio::time::advance(Duration::from_secs(4)).await;
    assert_eq!(
        perform(
            &mut c,
            click(f),
            Some(&Delay(Duration::from_secs(2))),
            &t,
            &cancel
        )
        .await,
        ActionReport::NotDispatched(Rejection::ApprovalDeadline)
    );
    assert_eq!(c.accounting().installed_receipts, 1);
    assert_eq!(c.accounting().admitted_actions, 0);
}

#[tokio::test(start_paused = true)]
async fn receipt_replacement_cannot_reset_counts_windows_or_text_limits() {
    let mut c = controller();
    let cancel = Cancellation::default();
    let t = Ack::default();
    for _ in 0..2 {
        let f = observation(&mut c, &cancel);
        let args = Arguments {
            frame_id: f,
            action: Action::Type {
                text: "é".repeat(512),
            },
        };
        assert_eq!(
            perform(&mut c, args, Some(&Yes), &t, &cancel).await,
            ActionReport::InputAcknowledged
        );
    }
    assert_eq!(c.accounting().admitted_actions, 2);
    assert_eq!(c.accounting().typed_bytes, 2048);
    assert_eq!(
        c.record_observation(
            ObservationMetadata {
                owner: c.owner.clone(),
                width: 1280,
                height: 800,
                admitted_at: Instant::now()
            },
            &cancel
        ),
        Err(Rejection::Limit)
    );
    assert_eq!(t.0.lock().unwrap().len(), 2);
    let mut c = controller();
    observation(&mut c, &cancel);
    observation(&mut c, &cancel);
    assert_eq!(
        c.record_observation(
            ObservationMetadata {
                owner: c.owner.clone(),
                width: 1280,
                height: 800,
                admitted_at: Instant::now()
            },
            &cancel
        ),
        Err(Rejection::Limit)
    );
    assert_eq!(c.accounting().installed_receipts, 2);
    let mut c = controller();
    observation(&mut c, &cancel);
    tokio::time::advance(RUN_WINDOW).await;
    assert_eq!(
        c.record_observation(
            ObservationMetadata {
                owner: c.owner.clone(),
                width: 1280,
                height: 800,
                admitted_at: Instant::now()
            },
            &cancel
        ),
        Err(Rejection::Limit)
    );
}

struct Pending {
    calls: AtomicUsize,
    entered: tokio::sync::Notify,
    dropped: Arc<AtomicBool>,
}
impl Default for Pending {
    fn default() -> Self {
        Self {
            calls: AtomicUsize::new(0),
            entered: tokio::sync::Notify::new(),
            dropped: Arc::new(AtomicBool::new(false)),
        }
    }
}
struct OnDrop(Arc<AtomicBool>);
impl Drop for OnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
#[async_trait]
impl ActionTransport for Pending {
    async fn act(&self, _: &ApprovalRequest) -> Result<(), TransportFailure> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let _guard = OnDrop(self.dropped.clone());
        self.entered.notify_one();
        std::future::pending().await
    }
}
struct Fail(AtomicUsize);
#[async_trait]
impl ActionTransport for Fail {
    async fn act(&self, _: &ApprovalRequest) -> Result<(), TransportFailure> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(TransportFailure)
    }
}

#[tokio::test(start_paused = true)]
async fn timeout_transport_error_and_dispatched_cancel_are_unknown_and_stop_run() {
    let cancel = Cancellation::default();
    let mut c = controller();
    let f = observation(&mut c, &cancel);
    let t = Pending::default();
    assert_eq!(
        perform(&mut c, click(f), Some(&Yes), &t, &cancel).await,
        ActionReport::Unknown(UnknownReason::Deadline)
    );
    assert_eq!(t.calls.load(Ordering::SeqCst), 1);
    assert!(t.dropped.load(Ordering::SeqCst));
    assert!(c.accounting().closed);
    assert_eq!(
        c.record_observation(
            ObservationMetadata {
                owner: c.owner.clone(),
                width: 1280,
                height: 800,
                admitted_at: Instant::now()
            },
            &cancel
        ),
        Err(Rejection::Closed)
    );
    assert_eq!(c.accounting().admitted_actions, 1);
    let mut c = controller();
    let f = observation(&mut c, &cancel);
    let fail = Fail(AtomicUsize::new(0));
    assert_eq!(
        perform(&mut c, click(f), Some(&Yes), &fail, &cancel).await,
        ActionReport::Unknown(UnknownReason::Transport)
    );
    assert_eq!(fail.0.load(Ordering::SeqCst), 1);
    assert!(c.accounting().closed);
    let mut c = controller();
    let f = observation(&mut c, &cancel);
    let t = Pending::default();
    let canceller = async {
        t.entered.notified().await;
        cancel.cancel();
    };
    let (result, ()) = tokio::join!(
        perform(&mut c, click(f), Some(&Yes), &t, &cancel),
        canceller
    );
    assert_eq!(result, ActionReport::Unknown(UnknownReason::Cancelled));
    assert!(c.accounting().closed);
    assert!(c.frame.is_none());
    assert_eq!(t.calls.load(Ordering::SeqCst), 1);
    assert!(t.dropped.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn dropping_admitted_perform_keeps_controller_closed_and_future_replies_cannot_reopen_it() {
    let mut c = controller();
    let cancel = Cancellation::default();
    let f = observation(&mut c, &cancel);
    let t = Pending::default();
    {
        let future = perform(&mut c, click(f), Some(&Yes), &t, &cancel);
        tokio::pin!(future);
        tokio::select! {biased; _=t.entered.notified()=>{}, _=&mut future=>panic!("pending mock completed")}
    }
    assert!(t.dropped.load(Ordering::SeqCst));
    assert!(c.accounting().closed);
    assert!(c.accounting().outcome_unknown);
    assert_eq!(c.accounting().admitted_actions, 1);
    assert!(c.frame.is_none());
    assert_eq!(
        c.record_observation(
            ObservationMetadata {
                owner: c.owner.clone(),
                width: 1280,
                height: 800,
                admitted_at: Instant::now()
            },
            &cancel
        ),
        Err(Rejection::Closed)
    );
}

#[test]
fn approval_is_complete_json_and_debug_reports_have_no_private_content() {
    let mut r = request();
    let text = "  \u{1b}[31m\u{85}\u{202e} café\n🦀\t  ";
    r.args.action = Action::Type { text: text.into() };
    let display = r.description();
    assert!(display.is_ascii());
    assert!(!display.contains('\u{1b}'));
    let value: serde_json::Value = serde_json::from_str(&display).unwrap();
    assert_eq!(value["arguments"]["action"]["text"], text);
    assert_eq!(value["machine_id"], r.owner.machine_id);
    assert_eq!(value["arguments"]["frame_id"], json!(r.args.frame_id));
    assert_eq!(value["limits"]["frame_age_seconds"], 5);
    let debug = format!(
        "{:?} {:?} {:?} {:?} {:?}",
        r,
        r.owner,
        r.args,
        ActionGrant::approve(r.clone(), RUN_WINDOW).unwrap(),
        ActionReport::Unknown(UnknownReason::Transport)
    );
    for sentinel in [
        text,
        r.owner.principal.as_str(),
        r.owner.machine_id.as_str(),
        r.provider_call_id.as_str(),
    ] {
        assert!(!debug.contains(sentinel));
    }
}

struct CancelWithAck(Cancellation);
#[async_trait]
impl ActionTransport for CancelWithAck {
    async fn act(&self, _: &ApprovalRequest) -> Result<(), TransportFailure> {
        self.0.cancel();
        Ok(())
    }
}

#[tokio::test(start_paused = true)]
async fn cancellation_in_ack_poll_is_unknown_not_completion() {
    let mut c = controller();
    let cancel = Cancellation::default();
    let f = observation(&mut c, &cancel);
    let transport = CancelWithAck(cancel.clone());
    assert_eq!(
        perform(&mut c, click(f), Some(&Yes), &transport, &cancel).await,
        ActionReport::Unknown(UnknownReason::Cancelled)
    );
    assert!(c.accounting().closed);
    assert!(c.accounting().outcome_unknown);
    assert_eq!(c.accounting().admitted_actions, 1);
}

#[tokio::test(start_paused = true)]
async fn invalid_input_precedes_approval_and_limits_apply_even_to_fresh_metadata() {
    struct MustNotAsk;
    #[async_trait]
    impl ActionApproval for MustNotAsk {
        async fn approve(&self, _: &ApprovalRequest) -> Option<ActionGrant> {
            panic!("invalid request reached broker")
        }
    }
    let cancel = Cancellation::default();
    let t = Ack::default();
    let mut c = controller();
    let f = observation(&mut c, &cancel);
    assert_eq!(
        perform(
            &mut c,
            Arguments {
                frame_id: f,
                action: Action::Type {
                    text: "é".repeat(513)
                }
            },
            Some(&MustNotAsk),
            &t,
            &cancel
        )
        .await,
        ActionReport::NotDispatched(Rejection::Arguments)
    );
    // Exercise budget guards independently of the stricter receipt cap. These
    // states model a future caller reaching the boundary, without changing limits.
    for text_limit in [false, true] {
        let mut c = controller();
        let f = observation(&mut c, &cancel);
        if text_limit {
            c.accounting.typed_bytes = MAX_RUN_TEXT;
        } else {
            c.accounting.admitted_actions = MAX_ACTIONS;
        }
        let args = Arguments {
            frame_id: f,
            action: Action::Type { text: "x".into() },
        };
        assert_eq!(
            perform(&mut c, args, Some(&MustNotAsk), &t, &cancel).await,
            ActionReport::NotDispatched(Rejection::Limit)
        );
    }
    let mut c = controller();
    let f = observation(&mut c, &cancel);
    c.close();
    assert_eq!(
        perform(&mut c, click(f), Some(&MustNotAsk), &t, &cancel).await,
        ActionReport::NotDispatched(Rejection::Closed)
    );
    let mut c = controller();
    let f = observation(&mut c, &cancel);
    cancel.cancel();
    assert_eq!(
        perform(&mut c, click(f), Some(&MustNotAsk), &t, &cancel).await,
        ActionReport::NotDispatched(Rejection::Cancelled)
    );
    assert!(t.0.lock().unwrap().is_empty());
    assert!(!c.accounting().outcome_unknown);
}

#[tokio::test(start_paused = true)]
async fn transport_wait_cannot_extend_the_original_run_window() {
    let mut c = controller();
    let cancel = Cancellation::default();
    observation(&mut c, &cancel);
    tokio::time::advance(Duration::from_secs(28)).await;
    let f = observation(&mut c, &cancel);
    let t = Pending::default();
    let before = Instant::now();
    assert_eq!(
        perform(&mut c, click(f), Some(&Yes), &t, &cancel).await,
        ActionReport::Unknown(UnknownReason::Deadline)
    );
    assert_eq!(
        Instant::now().duration_since(before),
        Duration::from_secs(2)
    );
    assert!(c.accounting().closed);
    assert_eq!(t.calls.load(Ordering::SeqCst), 1);
}
