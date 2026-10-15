# Maintenance and releases

## Ownership and review

The repository owner, [@BaileyPillon](https://github.com/BaileyPillon), maintains
the project and makes merge decisions. [CODEOWNERS](../.github/CODEOWNERS)
records repository ownership; it does not imply an independent review team.

Changes target `main`. Keep original behavior recovery, development tools, and
optional modifications explicit in each proposal and pull request. Follow
[CONTRIBUTING.md](../CONTRIBUTING.md) for reference evidence and validation.
Architectural changes should explain ownership, lifecycle, ordering, and the
tradeoff being made in the relevant subsystem document.

## Main branch policy

`main` requires a pull request and these successful, up-to-date checks:

- `Repository quality`
- `ubuntu-latest`
- `windows-latest`

The policy also applies to administrators. Force pushes and branch deletion
are disabled, conversations must be resolved, and merges preserve a linear
history. Use squash merge with a clear pull request title and description.

The project currently has one maintainer. The policy does not require an
unavailable second maintainer's approval. CODEOWNERS identifies who reviews
changes; it does not assert independent review or verification.

## Checks before merging

- Formatting, Clippy, headless tests, the optional audio tests, and the supported
  desktop compile check pass in [CI](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/actions/workflows/rust.yml).
- Documentation links and public JSON evidence pass `python tools/check_docs.py`.
- Changes to original behavior include an appropriate regression or comparison,
  with input identity, expected result, and exclusions recorded.
- Original files, credentials, and private evidence remain outside the commit.
- Status and progress claims describe the actual comparison scope. Frozen
  baselines remain intact; later evidence gets a dated progress entry.

Public CI uses owned fixtures and bounded numerical observations. Tests requiring
local original files or traces are explicitly ignored there and must be run
separately for changes that rely on those inputs.

## Issue and milestone maintenance

The [roadmap](ROADMAP.md#tracked-work) links scoped implementation issues to
three undated milestones. A milestone closes only after its recorded exit
criteria are met. Keep a task open when a component is implemented but its
stated original-game comparison remains incomplete.

Use area labels to route work and `parity` for original-behavior comparisons.
Use `good first issue` only after a task has a bounded implementation, usable
inputs, and clear acceptance criteria. Optional AI tools and mods have their
own workstream and do not contribute to original-game acceptance counts.

## Dependencies and automation

[Dependabot](../.github/dependabot.yml) proposes monthly updates for Cargo
dependencies and GitHub Actions. Minor and patch Cargo updates are grouped;
major upgrades remain separate. Dependency pull requests require the same
checks and review as other changes and are not automatically merged.

Keep `Cargo.lock` committed and builds locked. Review changes to Bevy, Rust,
audio, rendering, or scheduling for compatibility implications; update the
toolchain, manifests, setup guide, and affected reference checks together when
needed. GitHub Actions are pinned to commit SHAs. Dependency alerts and private
vulnerability reporting are enabled; follow [SECURITY.md](../SECURITY.md) for
security-sensitive findings.

CI runs on pull requests, pushes to `main`, and manual dispatch. Feature-branch
pushes use the pull request run to avoid duplicate builds of the same change.

## Release policy

There are currently no stable releases or supported release branches. The Cargo
workspace's development version is not a claim that a playable release exists.
Security fixes target `main`, as described in [SECURITY.md](../SECURITY.md).

A future playable release should identify its commit, supported platforms,
reference edition, tested controls and audio/display configuration. It needs
reproducible build/import instructions, an exercised gameplay path, clear known
limitations, and the required attribution. Distribute only project code and
permitted artifacts; original game files and BIOS images remain local inputs.

A playable preview may still have open compatibility work. Complete parity
requires the full [project objective](../PROJECT_OBJECTIVE.md), including
reviewed original/port scenarios across every provisional feature group and
resolved inventory gaps. A green build or callback count does not close those
gates.
