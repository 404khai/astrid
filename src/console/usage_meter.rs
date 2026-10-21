//! Known provider counts and per-metric coverage; context estimates stay separate.
use astrid::observability::Usage;

#[derive(Debug, Default)]
pub(super) struct UsageMeter {
    totals: [u128; 3],
    reporting: [usize; 3],
}

pub(super) fn call_label(usage: &Usage) -> String {
    let count = |value: Option<u64>| value.map_or_else(|| "unavailable".into(), |n| n.to_string());
    format!(
        "input {} · output {} · cached {} (subset of input){}",
        count(usage.input_tokens),
        count(usage.output_tokens),
        count(usage.cached_tokens),
        if usage.invalid {
            " · invalid provider usage"
        } else {
            ""
        },
    )
}

impl UsageMeter {
    pub(super) fn add(&mut self, usage: &Usage) {
        for (index, value) in [usage.input_tokens, usage.output_tokens, usage.cached_tokens]
            .into_iter()
            .enumerate()
        {
            if let Some(value) = value {
                self.totals[index] += u128::from(value);
                self.reporting[index] += 1;
            }
        }
    }

    pub(super) fn label(&self, calls: usize) -> String {
        ["input", "output", "cached"]
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                let reporting = self.reporting[index];
                let value = if reporting == 0 {
                    "unavailable".into()
                } else if reporting < calls {
                    format!("{} known", self.totals[index])
                } else {
                    self.totals[index].to_string()
                };
                format!("{name} {value} ({reporting}/{calls} calls)")
            })
            .collect::<Vec<_>>()
            .join(" · ")
    }
}
