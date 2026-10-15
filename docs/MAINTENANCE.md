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
