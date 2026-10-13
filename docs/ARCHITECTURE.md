# Architecture

The port keeps integer simulation separate from Bevy presentation. This makes
original-behavior comparisons possible without a GPU, wall-clock scheduling or
an AI account.

```mermaid
flowchart LR
    reference[Original PS1 observations] --> core[lsd-core]
    local[Locally supplied assets] --> assets[lsd-assets]
    core --> runtime[lsd-bevy]
    assets --> runtime
    tools[Optional AI analysis tools] -. reviewed artifacts .-> assets
```

`lsd-core` currently implements the PsyQ RNG, animation-word decoding,
inclusive mixed-width cell bounds and valid stream-layer ordering. Future
movement, linking, dream generation and NPC scripts belong here. Recovered
fixed-point widths and overflow rules take precedence over floating-point
convenience.

`lsd-assets` validates PS-X EXE headers and the canonical executable hash, and
decodes TIM/TIX, TMD polygon objects, LBD maps and MOM/MOS/MML model libraries.
Typed owned geometry and ordered animation records, plus bounded borrowed
texture blocks, replace PS1 file pointers. Animation execution, audio, movies
and save-data decoding remain open.

`lsd-bevy` exposes an explicitly stepped probe schedule and an optional desktop
preview and an original-stage asset viewer. Original simulation cadence,
interpolation and rendering quirks need
reference evidence before their implementation. Bevy presentation defaults
do not automatically establish PlayStation visual parity.

AI-assisted analysis and mod creation will produce reviewable artifacts. They
will not run inside the default compatibility logic. Original NPC scripts and
new AI-driven NPC mods have separate implementation and acceptance tracks.
