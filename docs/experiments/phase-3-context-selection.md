# Phase 3 context selection comparison

Date: 2026-10-07
Status: Deterministic retention comparison; not a task-success benchmark

Question: Under an allowance that fits only one older full exchange, does a file
explicitly named in the task survive differently under recency and file-reference
priority?

Fixture: task `repair target.rs`; older reads of `target.rs` and then `noise.rs`
with equally sized long results; newest protected read of `latest.rs`. Derive the
byte ceiling from one old exchange plus the protected task/latest exchange and
800 bytes of summary headroom. Both policies use the same provider serialization,
allowance, and original history. No model invocation or filesystem read is needed
for selection; provider accounting is pure and authentication panics if invoked.

| Policy | Full target.rs exchange | Full noise.rs exchange | Latest exchange |
| --- | --- | --- | --- |
| Recency | Omitted | Retained | Retained |
| File references + lexical path signals | Retained | Omitted | Retained |

The fixture asserts these results, request admission, and unchanged seven-message
history. Omitted full results can still contribute bounded summary excerpts;
that does not preserve their full contents. This establishes a difference in
raw evidence retention, not improved coding accuracy, latency, cost, or task
success. Repository dependency signals remain excluded from this initial slice.

Reproduce with:

```sh
cargo test --locked selection_compares_recency_and_file_relevance_without_reading_more_files
```

The phase review owns actual validation commands/results. No unmeasured efficiency
or model-success claim is made from this experiment.
