#[path = "support/day_state.rs"]
mod day_support;
#[path = "support/links.rs"]
#[allow(dead_code)]
mod support;
use day_support::*;
use lsd_core::{
    DayStartInputs, DreamDay, DreamDayEffects, DreamLinks, PlayerControls, SpawnLocation,
};
use serde_json::{Value, json};

struct Effects {
    actor: bool,
    cancel: bool,
    trace: Vec<Value>,
}
impl DreamDayEffects for Effects {
    fn notify(
        &mut self,
        _day: &mut DreamDay,
        links: &mut DreamLinks,
        _controls: &mut PlayerControls,
        code: i32,
    ) {
        self.trace.push(json!(["notify", code]));
        links.execution.state = if self.cancel { 0 } else { code };
    }
    fn attached_actor(&self) -> bool {
        self.actor
    }
    fn current_spawn(
        &mut self,
        _day: &mut DreamDay,
        _links: &mut DreamLinks,
        _controls: &mut PlayerControls,
    ) -> SpawnLocation {
        SpawnLocation {
            chunk: 257,
            tile: 513,
            position: [11, 22, 33],
        }
    }
}

#[test]
fn original_day_start_end_clock_flashback_and_retained_storage_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/day-clock.json")).unwrap();
    let tables = tables(&fixture["tables"]);
    let catalog = support::catalog(&fixture["catalog"]);
    for case in fixture["cases"].as_array().unwrap() {
        let (mut day, mut links, mut controls, mut context, mut rng) =
            initial(&fixture, &case["initial"]);
        let before = day.clone();
        let mut effects = Effects {
            actor: case["initial"]["actor"].as_bool().unwrap(),
            cancel: case["initial"]["cancel_notify"].as_bool().unwrap(),
            trace: Vec::new(),
        };
        let operation = case["operation"].as_str().unwrap();
        let argument = |i: usize| case["arguments"][i].as_i64().unwrap() as i32;
        let mut can_dynamic = true;
        let result = match operation {
            "start" => Some(
                day.start(
                    &mut links,
                    &mut controls,
                    DayStartInputs {
                        tables: &tables,
                        catalog: &catalog,
                        context: &mut context,
                        rng: &mut rng,
                    },
                    &mut effects,
                )
                .unwrap(),
            ),
            "end" => Some(day.end(&links, &tables, &mut context, argument(0)).unwrap()),
            "load" => Some(i32::from(
                day.load_flashback(&mut links, &mut controls, argument(0) == 0, &mut effects)
                    .unwrap(),
            )),
            "save" => {
                day.save_flashback(
                    &mut links,
                    &mut controls,
                    &mut rng,
                    argument(0),
                    argument(1),
                    &mut effects,
                )
                .unwrap();
                None
            }
            "init" => {
                day.init_new_game(&tables, &mut context);
                None
            }
            "tick" => {
                if day
                    .tick(
                        &mut links,
                        &mut controls,
                        &mut rng,
                        argument(1),
                        &mut effects,
                    )
                    .unwrap()
                {
                    if !controls.input_locked {
                        controls.action_pressed = false;
                        can_dynamic = links.execution.dream_timer.is_multiple_of(16);
                    }
                    effects.trace.extend([json!(["view"]), json!(["move"])]);
                }
                None
            }
            _ => unreachable!(),
        };
        let output = &case["output"];
        if let Some(result) = result {
            assert_eq!(result, num(output, "return"), "{operation}");
        }
        for (name, actual) in [
            ("year", day.year),
            ("day", day.day),
            ("stored_day", day.stored_day),
            ("total_unlock_score", day.total_unlock_score),
            ("navigation_score", day.navigation_score),
            ("instance_score", day.instance_score),
            ("flashback_count", day.flashback_count),
            ("flashback_index", day.flashback_index),
            ("is_new_game", day.is_new_game),
            ("screen_shake", i32::from(day.screen_shake)),
            ("state", links.execution.state),
            ("stage", links.execution.stage),
            ("limit", links.time_limit),
            ("penalty", context.dynamic_penalty),
            ("action_pressed", i32::from(controls.action_pressed)),
            ("can_dynamic", i32::from(can_dynamic)),
            ("reserved67c", day.reserved67c),
            ("reserved680", day.reserved680),
        ] {
            assert_eq!(
                actual,
                num(output, name),
                "{operation}: {name}, input={}",
                case["initial"]
            );
        }
        assert_eq!(
            day.save_magic,
            output["save_magic"].as_u64().unwrap() as u32
        );
        assert_eq!(
            links.execution.dream_timer,
            output["timer"].as_u64().unwrap() as u32
        );
        assert_eq!(
            rng.state(),
            output["rng_state"].as_u64().unwrap() as u32,
            "{operation}: {}",
            case["initial"]
        );
        assert_eq!(links.spawn, support::location(&output["spawn"]));
        assert_eq!(json!(context.navigation.to_vec()), output["navigation"]);
        assert_eq!(json!(day.cinematic), output["cinematic"]);
        assert_eq!(day.chunks, contributors(&output["chunks"]));
        assert_eq!(day.instances, contributors(&output["instances"]));
        assert_eq!(json!(day.reserved_flags), output["reserved_flags"]);
        assert_eq!(
            day.reserved684.iter().all(|&v| v == 0),
            output["reserved684_zero"].as_bool().unwrap()
        );
        let changed: Vec<_> = day
            .history
            .iter()
            .zip(&before.history)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, _))| json!([i, a.0]))
            .collect();
        assert_eq!(changed, *output["changed_history"].as_array().unwrap());
        let mut expected = before.flashbacks;
        for change in output["changed_records"].as_array().unwrap() {
            expected[change[0].as_u64().unwrap() as usize] = record(&change[1]);
        }
        assert_eq!(day.flashbacks, expected);
        assert_eq!(effects.trace, *output["trace"].as_array().unwrap());
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 793);
}
#[test]
fn original_previous_mood_year_boundary_and_chart_color_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/day-clock.json")).unwrap();
    let tables = tables(&fixture["tables"]);
    for case in fixture["mood_cases"].as_array().unwrap() {
        let (day, _, _, _, _) = initial(&fixture, &case["initial"]);
        let actual = if case["operation"] == "previous" {
            day.previous_mood(num(case, "mode") != 0).unwrap()
        } else {
            day.chart().unwrap()
        };
        assert_eq!(actual, mood(&case["mood"]));
        if case["operation"] == "chart" {
            assert_eq!(tables.color(actual), num(case, "color"));
        }
    }
    assert_eq!(fixture["mood_cases"].as_array().unwrap().len(), 48);
}
