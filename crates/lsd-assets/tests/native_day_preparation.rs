#[path = "../../lsd-core/tests/support/day_state.rs"]
#[allow(dead_code)]
mod day_support;
#[path = "../../lsd-core/tests/support/links.rs"]
#[allow(dead_code)]
mod support;
use lsd_assets::{load_day_tables, load_link_tables};
use lsd_core::{
    DayStartInputs, DreamDay, DreamDayEffects, DreamLinks, PlayerControls, SpawnLocation,
};
use serde_json::{Value, json};
use std::path::PathBuf;

struct Effects;
impl DreamDayEffects for Effects {
    fn notify(
        &mut self,
        _day: &mut DreamDay,
        links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        code: i32,
    ) {
        links.execution.state = code;
    }
    fn attached_actor(&self) -> bool {
        true
    }
    fn current_spawn(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
    ) -> SpawnLocation {
        SpawnLocation::default()
    }
}
#[test]
#[ignore = "Requires original executable and private original day observations"]
fn all_original_calendar_days_and_imported_content_match_native_startup() {
    let files = PathBuf::from(std::env::var_os("LSD_GAME_FILES").expect("Set LSD_GAME_FILES"));
    let input =
        PathBuf::from(std::env::var_os("LSD_NATIVE_DAY_CASES").expect("Set LSD_NATIVE_DAY_CASES"));
    let executable = std::fs::read(files.join("SLPS_015.56")).unwrap();
    let tables = load_day_tables(&executable).unwrap();
    let links = load_link_tables(&executable).unwrap();
    let fixture: Value = serde_json::from_slice(&std::fs::read(input).unwrap()).unwrap();
    assert_eq!(
        json!(tables.special_days.to_vec()),
        fixture["tables"]["special_days"]
    );
    assert_eq!(
        json!(tables.special_marker.0),
        fixture["tables"]["special_marker"]
    );
    assert_eq!(json!(tables.colors), fixture["tables"]["colors"]);
    assert_eq!(json!(tables.time_limits), fixture["tables"]["time_limits"]);
    assert_eq!(
        json!(tables.start_rotation),
        fixture["tables"]["start_rotation"]
    );
    assert_eq!(
        tables.save_magic,
        fixture["tables"]["save_magic"].as_u64().unwrap() as u32
    );
    for (i, grid) in tables.grids.iter().enumerate() {
        assert_eq!(
            json!({"columns":grid.columns,"rows":grid.rows,"mode":grid.mode}),
            fixture["tables"]["grids"][i]
        );
        assert_eq!(
            json!(
                tables.moods[i]
                    .iter()
                    .map(|mood| mood.0)
                    .collect::<Vec<_>>()
            ),
            fixture["tables"]["moods"][i]
        );
    }
    let mut special = 0;
    for case in fixture["cases"].as_array().unwrap() {
        let (mut day, mut runtime, mut controls, mut context, mut rng) =
            day_support::initial(&fixture, &case["initial"]);
        let result = day
            .start(
                &mut runtime,
                &mut controls,
                DayStartInputs {
                    tables: &tables,
                    catalog: &links.catalog,
                    context: &mut context,
                    rng: &mut rng,
                },
                &mut Effects,
            )
            .unwrap();
        let output = &case["output"];
        assert_eq!(result, day_support::num(output, "return"));
        assert_eq!(runtime.execution.stage, day_support::num(output, "stage"));
        assert_eq!(runtime.execution.state, day_support::num(output, "state"));
        assert_eq!(
            runtime.execution.dream_timer,
            output["timer"].as_u64().unwrap() as u32
        );
        assert_eq!(runtime.time_limit, day_support::num(output, "limit"));
        assert_eq!(runtime.spawn, support::location(&output["spawn"]));
        assert_eq!(rng.state(), output["rng_state"].as_u64().unwrap() as u32);
        assert_eq!(day.chunks, day_support::contributors(&output["chunks"]));
        assert_eq!(
            day.instances,
            day_support::contributors(&output["instances"])
        );
        assert_eq!(json!(day.cinematic), output["cinematic"]);
        assert_eq!(day.stored_day, day_support::num(output, "stored_day"));
        assert_eq!(context.dynamic_penalty, day_support::num(output, "penalty"));
        if result < 0 {
            special += 1;
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 1460);
    assert_eq!(special, 164);
}
