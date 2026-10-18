//! Presentation of admission metadata, never a claim about the model's window.
use astrid::context::{ContextBudget, ContextSelection, ContextSnapshot};

#[derive(Debug, Default)]
pub(super) struct ContextMeter {
    snapshot: Option<ContextSnapshot>,
    budget: Option<ContextBudget>,
}
impl ContextMeter {
    pub(super) fn select(&mut self, selection: &ContextSelection) {
        self.snapshot = Some(selection.snapshot.clone());
        self.budget = Some(selection.budget.clone());
    }
    pub(super) fn prepared(&mut self, snapshot: &ContextSnapshot) {
        self.snapshot = Some(snapshot.clone());
    }
    pub(super) fn label(&self) -> String {
        let Some(snapshot) = &self.snapshot else {
            return "ctx unavailable".into();
        };
        let partial = snapshot.measurements.iter().any(|m| m.contains_opaque_data);
        let kind = if partial { "partial est" } else { "est" };
        let used = snapshot.non_opaque_json_size_token_heuristic;
        match &self.budget {
            Some(budget) => {
                let allowance = budget
                    .estimated_context_tokens
                    .saturating_sub(budget.response_reserve_tokens);
                if allowance == 0 {
                    return format!("ctx {kind} ~{used}/? budget (?%)");
                }
                let percent = used as u128 * 100 / allowance as u128;
                format!("ctx {kind} {percent}% ~{used}/{allowance} budget")
            }
            None => format!("ctx {kind} ?% ~{used}/? budget"),
        }
    }
    pub(super) fn line(&self) -> String {
        let limits = self.budget.as_ref().map_or_else(String::new, |budget| {
            format!(
                " · reserve {} (planning only) · request bytes {}/{}",
                budget.response_reserve_tokens,
                self.snapshot
                    .as_ref()
                    .map_or(0, |s| s.serialized_request_bytes),
                budget.max_request_bytes,
            )
        });
        format!(
            "  {} · non-opaque bytes/4; model-window occupancy unknown{}",
            self.label(),
            limits
        )
    }
}
pub(super) fn idle_label(selection: Option<&ContextSelection>) -> String {
    let mut meter = ContextMeter::default();
    if let Some(selection) = selection {
        meter.select(selection);
    }
    format!("last {}", meter.label())
}
