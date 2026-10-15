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

## Development workflow

For external contributions, fork the repository on GitHub and clone your fork.
Keep the project repository as `upstream`:

```sh
git remote add upstream https://github.com/BaileyPillon/lsd-dream-emulator-bevy.git
git fetch upstream
git switch -c fix/describe-the-change upstream/main
```

Push the branch to your fork and open a pull request targeting this project's
`main`. Maintainers working in the project repository can use the workflow
below directly.

1. Choose an existing [roadmap task](docs/ROADMAP.md#tracked-work), or open a
   proposal for a new behavior or substantial design change. Describe the
   expected result and how it will be checked.
2. Create a focused branch from current `main`:

   ```sh
   git switch main
   git pull --ff-only
   git switch -c fix/describe-the-change
   ```

3. Implement the change in the appropriate crate and run the relevant checks
   below. For behavior recovery, use the
   [compatibility report template](docs/templates/COMPATIBILITY_REPORT.md).
4. Open a pull request with reproduction steps, results, and remaining gaps.
   The required CI checks must pass before merging. Maintainers squash merge
   focused pull requests; do not rewrite published `main` history.

## Find the code

| Change | Implementation | Validation |
| --- | --- | --- |
| Deterministic simulation, NPC scripts, collision, or SPU state | `crates/lsd-core/src/` | `crates/lsd-core/tests/` and module tests |
| File formats, import, or original-content scene binding | `crates/lsd-assets/src/` | `crates/lsd-assets/tests/` and owned format fixtures |
| Rendering, input, desktop inspectors, or host audio | `crates/lsd-bevy/src/` | `crates/lsd-bevy/tests/` and an exercised desktop scenario |
| Reference observations and progress claims | `docs/` | Bounded check manifests and `python tools/check_docs.py` |
| Optional AI-assisted analysis | `tools/ai-assist/` | Explicit tool inputs, outputs, provenance, and isolation from default gameplay |

The [architecture guide](docs/ARCHITECTURE.md) explains dependencies and ownership.
The [developer reference](docs/DEVELOPMENT.md) maps every executable, Cargo
feature, and common test command to its purpose.

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
Documentation-only changes do not need new Rust tests. Run targeted behavior
tests while iterating; the full required matrix runs on the pull request.

## Evidence standards

Use the [compatibility issue form](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/issues/new?template=compatibility.yml)
for a specific original/port difference. Reports may include incomplete
evidence when they clearly identify what remains untested.

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
