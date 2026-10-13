//! The Bevy host for deterministic probes. Original simulation stays in lsd-core.
use bevy::{ecs::schedule::ScheduleLabel, prelude::*};
use lsd_core::CompatibilityProbe;

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProbeStep;

#[derive(Resource, Debug, Default)]
pub struct ProbeState(pub CompatibilityProbe);

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
