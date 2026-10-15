use lsd_core::{SequenceAdvanceFault, SequenceClock, SequenceClockFault};
use serde_json::Value;
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/sequence-clock.json")).unwrap()
}
fn clock(v: &Value) -> SequenceClock {
    let n = |key: &str| v[key].as_i64().unwrap();
    SequenceClock {
        countdown: n("countdown") as i16,
        step: n("step") as i16,
        initial_step: n("initial_step") as i16,
        remaining: n("remaining") as i32,
    }
}
#[test]
fn initialization_matches_original_sdk_rounding_widths_and_division_faults() {
    for row in fixture()["initialization"].as_array().unwrap() {
        let result = SequenceClock::initialize(
            row["resolution"].as_i64().unwrap() as i16,
            row["tempo"].as_u64().unwrap() as u32,
            row["rate"].as_u64().unwrap() as u32,
            row["initial_delta"].as_u64().unwrap() as u32,
        );
        if let Some(pc) = row["fault_pc"].as_str() {
            let expected = match pc {
                "0x3a800" => SequenceClockFault::ZeroTempo,
                "0x3a8dc" => SequenceClockFault::ZeroResolutionProduct,
                "0x3a920" => SequenceClockFault::ZeroCallbackRate,
                _ => panic!("Unclassified original fault {pc}"),
            };
            assert_eq!(result, Err(expected), "{row}");
        } else {
            let (actual, bpm) = result.unwrap();
            assert_eq!(actual, clock(&row["clock"]), "{row}");
            assert_eq!(u64::from(bpm), row["bpm"].as_u64().unwrap());
        }
    }
}
#[test]
fn callback_branches_and_event_boundaries_match_actual_original_dispatch() {
    for (index, row) in fixture()["ticks"].as_array().unwrap().iter().enumerate() {
        let mut actual = clock(&row["before"]);
        let mut events = Vec::new();
        let deltas = row["deltas"].as_array().unwrap();
        let mut cursor = 0;
        let result = actual
            .advance(32, |clock| {
                events.push(*clock);
                clock.remaining = (deltas[cursor % deltas.len()].as_u64().unwrap() as u32)
                    .wrapping_mul(10) as i32;
                cursor += 1;
                Ok::<_, ()>(())
            })
            .unwrap();
        assert_eq!(actual, clock(&row["after"]), "case {index}");
        assert_eq!(
            i64::from(result),
            row["return_value"].as_i64().unwrap(),
            "case {index}"
        );
        let expected: Vec<_> = row["dispatch_before"]
            .as_array()
            .unwrap()
            .iter()
            .map(clock)
            .collect();
        assert_eq!(events, expected, "case {index}");
    }
}
#[test]
fn tempo_events_preserve_the_original_distinct_truncation_and_saved_step() {
    for row in fixture()["tempo"].as_array().unwrap() {
        let mut actual = clock(&row["before"]);
        let result = actual.set_tempo(
            row["resolution"].as_i64().unwrap() as i16,
            row["tempo"].as_u64().unwrap() as u32,
            row["rate"].as_u64().unwrap() as u32,
        );
        if let Some(pc) = row["fault_pc"].as_str() {
            let expected = match pc {
                "0x35bec" => SequenceClockFault::ZeroTempo,
                "0x35c6c" => SequenceClockFault::ZeroResolutionProduct,
                "0x35cb0" => SequenceClockFault::ZeroCallbackRate,
                _ => panic!("Unclassified original tempo fault {pc}"),
            };
            assert_eq!(result, Err(expected), "{row}");
            assert_eq!(actual, clock(&row["before"]));
        } else {
            assert_eq!(u64::from(result.unwrap()), row["bpm"].as_u64().unwrap());
            actual.remaining = (row["next_delta"].as_u64().unwrap() as u32).wrapping_mul(10) as i32;
            assert_eq!(actual, clock(&row["after"]), "{row}");
        }
    }
}
#[test]
fn incomplete_event_dispatch_and_zero_delta_limits_retain_the_last_state() {
    let mut state = SequenceClock {
        countdown: -1,
        step: 107,
        initial_step: 107,
        remaining: 0,
    };
    let mut calls = 0;
    assert_eq!(
        state.advance(3, |clock| {
            calls += 1;
            clock.remaining = 0;
            Ok::<_, ()>(())
        }),
        Err(SequenceAdvanceFault::Clock(SequenceClockFault::EventLimit))
    );
    assert_eq!(calls, 3);
    assert_eq!(state.remaining, 0);
    assert_eq!(
        state.advance(3, |clock| {
            clock.remaining = 17;
            Err("unimplemented event")
        }),
        Err(SequenceAdvanceFault::Event("unimplemented event"))
    );
    assert_eq!(state.remaining, 17);
}
