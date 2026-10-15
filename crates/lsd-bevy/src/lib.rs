//! Bevy integration, development probes, and desktop presentation for LSD: Dream Emulator.
//!
//! Original simulation stays in [`lsd_core`]; local content loading and scene
//! adapters stay in [`lsd_assets`]. This crate hosts them in Bevy and provides
//! inspectors, input, rendering, and optional audio output.
//!
//! # Cargo features
//!
//! | Feature | Purpose |
//! | --- | --- |
//! | Default (none) | Headless compatibility probe; no game files or audio device required |
//! | `audio` | Continuous host audio output and the live audio diagnostic |
//! | `desktop` | Rendering, window/input integration, and original-content inspectors; includes `audio` |
//!
//! Generate documentation with `--features desktop` to include the feature-gated
//! scene, audio, and screen-box modules. The default API documentation covers the
//! headless host.
//!
//! # Run a deterministic probe
//!
//! ```
//! let result = lsd_bevy::run_probe(1, 120);
//! assert_eq!(result.steps, 120);
//! assert_eq!(result, lsd_bevy::run_probe(1, 120));
//! ```
//!
//! Probe steps exercise development state; they do not represent a complete
//! dream session. To explore the original-content prototype, use the `lsd-stage`
//! binary's `--dream` profile with your own extracted files. See
//! [getting started](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/blob/main/docs/GETTING_STARTED.md)
//! for its command, controls, and current limitations, and the
//! [developer reference](https://github.com/BaileyPillon/lsd-dream-emulator-bevy/blob/main/docs/DEVELOPMENT.md)
//! for all binaries and supported feature combinations.
use bevy::{ecs::schedule::ScheduleLabel, prelude::*};
use lsd_core::CompatibilityProbe;

#[cfg(feature = "desktop")]
pub mod asset_scene;
#[cfg(feature = "audio")]
pub mod audio;
#[cfg(feature = "desktop")]
pub mod box_material;

/// Schedule for one explicit development-probe step.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProbeStep;

/// Probe state owned by the headless Bevy host.
#[derive(Resource, Debug, Default)]
pub struct ProbeState(pub CompatibilityProbe);

/// Installs the probe resource, schedule, and deterministic update system.
pub struct CompatibilityProbePlugin;

impl Plugin for CompatibilityProbePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProbeState>()
            .add_schedule(Schedule::new(ProbeStep))
            .add_systems(ProbeStep, advance_probe);
    }
}

fn advance_probe(mut probe: ResMut<ProbeState>) {
    probe.0.step();
}

/// Runs a fresh headless probe for the requested steps and returns its final state.
///
/// Uses Bevy's minimal plugins without a window, original files, or an audio device.
/// These steps are development ticks, not original-game frame observations.
pub fn run_probe(seed: u32, steps: usize) -> CompatibilityProbe {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(ProbeState(CompatibilityProbe::new(seed)))
        .add_plugins(CompatibilityProbePlugin);
    for _ in 0..steps {
        app.world_mut().run_schedule(ProbeStep);
    }
    app.world().resource::<ProbeState>().0.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_schedule_matches_core_without_wall_clock_dependence() {
        for seed in [0, 1, 0x8000_0000, u32::MAX] {
            let mut expected = CompatibilityProbe::new(seed);
            for _ in 0..120 {
                expected.step();
            }
            assert_eq!(run_probe(seed, 120), expected);
        }
    }

    #[test]
    fn separate_apps_do_not_share_rng_state() {
        assert_eq!(run_probe(7, 0), CompatibilityProbe::new(7));
        assert_eq!(run_probe(7, 17), run_probe(7, 17));
        assert_ne!(run_probe(7, 17), run_probe(8, 17));
    }
}
