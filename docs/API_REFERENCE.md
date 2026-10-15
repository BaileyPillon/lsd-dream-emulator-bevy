# API reference

Browse the public Rust interfaces without installing the toolchain or supplying
game files. The reference is generated from the same `main` revision as this
handbook, with Rustdoc warnings treated as errors.

| Library | Start here | Responsibility |
| --- | --- | --- |
| `lsd-core` | [Simulation API](https://baileypillon.github.io/lsd-dream-emulator-bevy/api/lsd_core/index.html) | Deterministic math, RNG, dream sessions, NPC scripts, collision, animation, and software SPU state |
| `lsd-assets` | [Content API](https://baileypillon.github.io/lsd-dream-emulator-bevy/api/lsd_assets/index.html) | Local format readers, reference validation, content binding, and scene adapters |
| `lsd-bevy` | [Host API](https://baileypillon.github.io/lsd-dream-emulator-bevy/api/lsd_bevy/index.html) | Headless Bevy integration and the compatibility probe |

Each crate overview includes entry points and an example. Rustdoc provides type
search, source navigation, and links between the three libraries. The
[build identity](https://baileypillon.github.io/lsd-dream-emulator-bevy/api/build.json)
records the exact source commit and documented feature profile.

## Scope

The published reference covers library targets with **default Cargo features**.
The optional desktop and host-audio APIs are available in a local build after
installing their [platform dependencies](GETTING_STARTED.md#requirements).
The [developer reference](DEVELOPMENT.md#check-a-pull-request) explains both
documentation commands and the executable tools.

The APIs are under active development and may change before a stable release.
Documentation builds and examples verify interfaces; original-game acceptance
is tracked separately in [project status](STATUS.md).

## Build locally

```sh
cargo doc --workspace --lib --no-deps --locked
```

Open `target/doc/lsd_core/index.html`, `target/doc/lsd_assets/index.html`, or
`target/doc/lsd_bevy/index.html`. A custom `CARGO_TARGET_DIR` changes the output
root. To include optional desktop APIs:

```sh
cargo doc -p lsd-bevy --lib --no-deps --features desktop --locked
```

See the [documentation workflow](DOCUMENTATION.md) for a combined handbook and
API preview, deployment checks, and ownership.
