# Try Phase 2

Run the offline checks and install the current CLI from the Astrid repository:

```sh
cd /Users/admin/Developer/astrid
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo install --path . --locked --force
```

For just the new execution-environment suites:

```sh
cargo test --locked --test environment --test native_bounds --test changes --test phase2 --test phase2_adversarial
```

Set a model available to your signed-in account. `astrid login` establishes a
session if needed; `astrid models` lists the catalog. The live acceptance used
`gpt-5.6-luna`:

```sh
astrid models
export ASTRID_MODEL=gpt-5.6-luna
```

## Create a disposable, dirty Git fixture

Run this from the Astrid repository. It copies the greeting fixture, creates a
local baseline, and leaves USER.txt with separate staged and unstaged edits:

```sh
cd /Users/admin/Developer/astrid
phase2_fixture=$(mktemp -d)
cp -R tests/fixtures/greeting/. "$phase2_fixture/"
cd "$phase2_fixture"
git init -q
printf 'original user file\n' > USER.txt
git add .
git -c user.name='Astrid Fixture' -c user.email='fixture@example.invalid' -c commit.gpgsign=false commit -qm 'fixture baseline'
printf 'staged user edit\n' > USER.txt
git add USER.txt
printf 'unstaged user edit to preserve\n' > USER.txt
```

## Repair, permissions, and change reporting

```sh
astrid run "Inspect the repository, run its failing tests, fix the greeting bug without changing tests, create GREETING.txt, and rerun tests. Preserve USER.txt content and staged state." --shell-policy ask
```

Type `yes` for the test commands you approve. Expect the initial two tests to
fail and the final two to pass. The CLI shows the initial dirty file, operation
permission decisions, native patches, and changes relative to the starting
workspace. Independently verify the result:

```sh
cargo test --offline
cat USER.txt
git show :USER.txt
cat GREETING.txt
git diff -- src/lib.rs
```

USER.txt should still contain `unstaged user edit to preserve`; the staged version
should contain `staged user edit`. GREETING.txt should contain `Hello, Astrid!`
and a newline.

## Live stdout and stderr

```sh
astrid run "Run one shell command that prints stream-first to stdout, stream-error to stderr, waits three seconds, then prints stream-last to stdout. Do not modify files." --shell-policy ask
```

Approve the command. The first two messages should appear before the wait ends;
stdout/stderr are labeled separately. The completion summary does not repeat
the streamed output.

## Denied native writes and shell execution

```sh
astrid run "Attempt to create DENIED.txt using write_file and SHELL-DENIED.txt using shell. Report any permission denials without retrying." --write-policy deny --shell-policy deny
test ! -e DENIED.txt
test ! -e SHELL-DENIED.txt
```

The effective policies should show Deny, and neither file should exist. A model
may decline to request a prohibited tool; the deterministic permission matrix
tests exercise actual requests and prove that denial bypasses execution.

## Timeout and cancellation

```sh
astrid run "Run one shell command that prints before-timeout, then sleeps for 30 seconds. Report its tool outcome without retrying." --shell-timeout 2 --shell-policy ask
```

Approve it. The tool should time out after two seconds, retain observed output,
and display incomplete-output metadata. The model can then finish normally;
normal run termination does not erase the tool's timeout.

```sh
astrid run "Run one shell command that prints before-cancel, then sleeps for 60 seconds." --shell-timeout 60 --shell-policy ask
```

Approve it, wait for `before-cancel`, then press **Ctrl-C**. Expect a cancelled run,
retained partial-output metadata, final change evidence, and a nonzero CLI exit.

## Output limits

```sh
astrid run "Run one shell command that prints 200000 x characters to stdout and 200000 e characters to stderr. Report the output coverage metadata without repeating the output." --shell-policy ask --max-model-calls 2
```

Approve the finite command. Expect at most a 64 KiB captured prefix per stream,
with observed/captured/live omission counters. Live admission can stop earlier
under consumer pressure; omitted bytes remain explicit. For quiet deterministic
verification of limits and stalled-consumer cleanup:

```sh
cd /Users/admin/Developer/astrid
cargo test --locked --test phase2 finite_dual_pipe_flood_caps_capture_and_counts_observed_bytes
cargo test --locked --test environment shell_cleanup_precedes_resuming_a_stalled_event_consumer
```

Permission flags govern native tools and broad shell execution. `ask` allows you
to inspect each command; `--shell-policy allow` explicitly skips those prompts
and grants execution with your account's permissions. Runtime instruction and
evidence collection remain bookkeeping reads. Full guarantees and limits are in
the [Phase 2 review](reviews/phase-2.md).
