# Contributing

Start with the [roadmap](docs/ROADMAP.md) and choose a bounded behavior or asset
format. Explain the original evidence and the expected user-visible result.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p lsd-bevy --features audio --locked
cargo check -p lsd-bevy --features desktop --locked
```

The desktop target needs platform dependencies; the default workspace tests
are headless. CI tests Ubuntu and Windows and checks the desktop target on Windows.
On Debian/Ubuntu, install `libasound2-dev` before checking audio or desktop
features. Audio-source tests need no output device; live playback does.

## Evidence and parity

Use explicit integer widths and wrapping operations where original behavior
requires them. Record reference identity, initial state, inputs, outputs and
scope for comparisons. Keep invented fixtures distinct from captured gameplay.
Never count a preview, source translation or partial trace as an accepted
whole-game feature. Update the fresh port tracker without rewriting frozen
C-project measurements.

Keep original scripts and compatibility behavior in `lsd-core`. New AI-driven
NPC behaviors belong to optional mods; AI analysis tools belong under
`tools/ai-assist`. Core tests must run offline without accounts or model keys.

Commit only code, documentation and small synthetic/reference-observation
fixtures. Keep local binaries, extracted assets, full memory captures, keys and
analysis databases in ignored storage. Describe validation and remaining gaps
in each pull request.
