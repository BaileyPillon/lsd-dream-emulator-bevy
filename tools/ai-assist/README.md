# AI-assisted development tools

This is the reserved workflow area for future AI-assisted asset analysis,
behavior documentation and mod authoring. Data replacement packs are implemented separately in the [modding guide](../../docs/MODDING.md). No model integration is implemented
or configured in the initial port.

Original NPC behavior belongs in `lsd-core` as deterministic recovered scripts.
New AI-driven NPCs belong to optional mods. An analysis tool must preserve its
input hashes, output provenance and human-review boundary, and must never
silently change canonical gameplay or the original parity scorecard.

The companion C project's tool applicability research remains a reference.
Installing or naming an analysis tool does not count as an implemented original
game feature. Keep model credentials and original assets outside Git.
