# Contributing

Thank you for helping preserve LSD: Dream Emulator. Start with
[project status](docs/STATUS.md), [the roadmap](docs/ROADMAP.md), and
[the build guide](docs/GETTING_STARTED.md).

## Choose a contribution

Useful work includes original behavior recovery, the remaining NPC scripts,
content import, rendering, audio, UI/media/persistence, comparison scenarios,
and clearer documentation. Choose a bounded problem and describe the expected
user-visible result before undertaking a large change.

Use a [feature proposal](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/new?template=feature.yml)
for implementation work or a [bug report](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/new?template=bug_report.yml)
for a reproducible problem. Search existing issues first.

## Development checks

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p lsd-bevy --features audio --locked
cargo check -p lsd-bevy --features desktop --locked
```

The workspace pins its toolchain and Cargo lockfile. Default tests are headless;
the optional audio and desktop features need their platform dependencies.
CI runs Ubuntu and Windows and checks the desktop target on Windows.
For documentation or public evidence changes, also run
`python tools/check_docs.py` with Python 3.10 or newer.

## Evidence standards

For behavior changes, record reference identity, input hashes, initial state,
inputs, outputs, timing assumptions, and comparison scope. Use explicit integer
widths and wrapping where the original requires them. Include a meaningful
regression check when behavior changes.

Distinguish owned fixtures, original-code component observations, controlled
integrations, and natural original-game scenarios. Do not count a preview,
source translation, or partial trace as a complete accepted feature. Update
dated progress evidence without rewriting frozen C or initial Rust measurements.

Original-file tests are explicitly ignored unless local input paths are
provided. Run only the relevant ignored tests with the assets/traces they name;
do not use a blanket `--include-ignored` command on a machine without those inputs.

## Repository boundaries

- Original simulation and scripts belong in `lsd-core`; engine integration
  belongs in `lsd-bevy`; local import and binding belong in `lsd-assets`.
- Keep original game files, BIOS images, save states, full memory captures,
  audio renders, analysis databases, and credentials outside version control.
- Public fixtures must use owned content or appropriate bounded observations.
  Keep large or original-content evidence local and publish hashes and scope.
- AI analysis tools belong under `tools/ai-assist`. AI-driven gameplay changes
  belong to optional mods. Original behavior and core tests require no accounts.
- Preserve provenance and third-party attribution in [NOTICE.md](NOTICE.md).

## Pull requests

Explain the problem, resulting behavior, original evidence, verification, and
remaining limitations. Keep changes focused and provide reproduction commands.
Mark intentional gameplay differences as optional. A successful CI run verifies
its checks; it does not establish full original-game parity.

Contributors must follow the [code of conduct](CODE_OF_CONDUCT.md).
For security-sensitive reports, follow [SECURITY.md](SECURITY.md).
See [maintenance and releases](docs/MAINTENANCE.md) for ownership, merge checks,
and the distinction between a development milestone and a playable release.
