# Compatibility report: <behavior or scenario>

Copy this template into a pull request or a new subsystem note. Replace the
placeholders and remove sections that do not apply. Do not attach original
game files, full memory dumps, BIOS images, or private analysis databases.

## Result and scope

Describe the observable behavior being recovered and the result in one
paragraph. State whether the evidence covers an individual function, a
controlled integration, or a complete natural game scenario.

| Field | Value |
| --- | --- |
| Port revision | Full commit SHA, including whether local changes are present |
| Reference identity | Game edition, executable SHA-256, relevant content hashes |
| Reference execution | Original hardware, emulator/core and version, or native component runner |
| Relevant functions and formats | Addresses, symbols, structures, or source anchors |
| Starting state | Scenario, map, day, seed, and relevant state identity |
| Input and timing | Controller/input sequence, frame count, clock assumptions |
| Reproduction | Exact commands, required local inputs, comparison tool revision |

## Acceptance criteria

Define the comparison before assessing its result: which state, image, audio,
event ordering, or timing must match, and what tolerances apply and why.
Identify any injected state, mocked dependency, masked field, or substituted
output. If none are used, say so explicitly.

## Observations

| Check | Original result | Port result | Outcome and evidence |
| --- | --- | --- | --- |
| <specific observable> | <result> | <result> | <pass/fail and bounded artifact or hash> |

Report the actual number of cases executed, including failed and skipped
cases. A test count measures only the stated scope. Link public owned fixtures
or bounded numeric observations; record hashes and local reproduction
instructions for evidence that cannot be distributed.

## Limitations and follow-up

Record remaining mismatches, untested branches, platform differences, and
excluded behavior. Identify the next test needed to close each material gap.
A component match does not accept an entire encounter or feature group.

If the report changes a progress claim, identify the dated entry added to
[parity.json](../parity.json) and the corresponding [status](../STATUS.md)
update. Preserve the frozen initial Rust baseline and C reference checkpoint.
