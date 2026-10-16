//! Explicit direct-inference routing. This does not delegate to an agent harness.
use crate::model::{ModelError, ModelProvider, ModelRequest, ModelResponse, TextSink};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffort {
    #[serde(rename = "none")]
    #[value(name = "none")]
    Disabled,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapability {
    pub visible_history: bool,
    /// Empty means explicit effort is unsupported or unknown, never a guessed default.
    pub reasoning_efforts: Vec<ReasoningEffort>,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayMode {
    #[default]
    ProviderContinuation,
    VisibleHistory,
}
/// Only validated dispatch configuration; no claim about server-side resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchOrigin {
    pub provider: Option<String>,
    pub model: String,
    pub effort: Option<ReasoningEffort>,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayReason {
    #[default]
    Unknown,
    NativeContinuation,
    ConfiguredStages,
    UnverifiedHistory,
    ModelOrProviderChanged,
    SessionVisibleHistory,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct RequestOptions {
    pub effort: Option<ReasoningEffort>,
    pub replay: ReplayMode,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelClass {
    #[default]
    General,
    Retrieval,
    Coding,
    Reasoning,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRoute {
    /// Switch only after the previous call's entire tool batch is terminal.
    pub from_call: usize,
    pub model: String,
    pub class: ModelClass,
    pub effort: Option<ReasoningEffort>,
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingPolicy {
    pub initial_effort: Option<ReasoningEffort>,
    #[serde(default)]
    pub stages: Vec<ModelRoute>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteReason {
    FixedModel,
    ConfiguredInitial,
    ConfiguredStage { from_call: usize },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDecision {
    pub provider: Option<String>,
    pub requested_model: String,
    /// Validated request configuration, not a claim about unreported server resolution.
    pub effective_model: String,
    pub requested_effort: Option<ReasoningEffort>,
    pub effective_effort: Option<ReasoningEffort>,
    pub class: ModelClass,
    pub reason: RouteReason,
    pub replay: ReplayMode,
    #[serde(default)]
    pub replay_reason: ReplayReason,
}
impl RoutingPolicy {
    pub fn valid(&self, initial: &str, ceiling: usize, provider: &dyn ModelProvider) -> bool {
        let supported = |model: &str, effort: Option<ReasoningEffort>| {
            effort.is_none_or(|effort| {
                provider
                    .capabilities(model)
                    .reasoning_efforts
                    .contains(&effort)
            })
        };
        let mut previous = 1;
        (self.stages.is_empty() || provider.capabilities(initial).visible_history)
            && self.stages.len() <= 32
            && supported(initial, self.initial_effort)
            && self.stages.iter().all(|stage| {
                let valid = stage.from_call > previous
                    && stage.from_call <= ceiling
                    && !stage.model.trim().is_empty()
                    && stage.model.len() <= 256
                    && !stage.model.chars().any(char::is_control)
                    && provider.capabilities(&stage.model).visible_history
                    && supported(&stage.model, stage.effort);
                previous = stage.from_call;
                valid
            })
    }
}
pub(crate) fn select(
    provider: &dyn ModelProvider,
    initial: &str,
    policy: Option<&RoutingPolicy>,
    number: usize,
) -> RouteDecision {
    let stage = policy.and_then(|p| p.stages.iter().rev().find(|s| s.from_call <= number));
    let (model, effort, class, reason) = if let Some(stage) = stage {
        (
            stage.model.as_str(),
            stage.effort,
            stage.class,
            RouteReason::ConfiguredStage {
                from_call: stage.from_call,
            },
        )
    } else {
        (
            initial,
            policy.and_then(|p| p.initial_effort),
            ModelClass::General,
            if policy.is_some() {
                RouteReason::ConfiguredInitial
            } else {
                RouteReason::FixedModel
            },
        )
    };
    RouteDecision {
        provider: provider.provider_name().map(str::to_owned),
        requested_model: model.into(),
        effective_model: model.into(),
        requested_effort: effort,
        effective_effort: effort,
        class,
        reason,
        replay_reason: if policy.is_some_and(|p| !p.stages.is_empty()) {
            ReplayReason::ConfiguredStages
        } else {
            ReplayReason::NativeContinuation
        },
        replay: if policy.is_some_and(|p| !p.stages.is_empty()) {
            ReplayMode::VisibleHistory
        } else {
            ReplayMode::ProviderContinuation
        },
    }
}
/// Bind exactly the same options during selection, preflight and dispatch.
pub(crate) struct RequestProvider<'a> {
    pub inner: &'a dyn ModelProvider,
    pub options: RequestOptions,
}
#[async_trait]
impl ModelProvider for RequestProvider<'_> {
    fn provider_name(&self) -> Option<&'static str> {
        self.inner.provider_name()
    }
    fn capabilities(&self, model: &str) -> ModelCapability {
        self.inner.capabilities(model)
    }
    fn measure_request(
        &self,
        request: &ModelRequest<'_>,
    ) -> Result<Option<crate::context::ContextSnapshot>, ModelError> {
        self.inner.measure_request(&ModelRequest {
            options: self.options,
            ..*request
        })
    }
    async fn generate(
        &self,
        request: &ModelRequest<'_>,
        text: &mut dyn TextSink,
    ) -> Result<ModelResponse, ModelError> {
        self.inner
            .generate(
                &ModelRequest {
                    options: self.options,
                    ..*request
                },
                text,
            )
            .await
    }
}
