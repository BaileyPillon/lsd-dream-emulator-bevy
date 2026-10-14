use lsd_core::{DreamSession, DreamSessionHost};
use serde_json::{Value, json};
use std::convert::Infallible;

struct Host<'a> {
    input: &'a Value,
    trace: Vec<Value>,
}
impl DreamSessionHost for Host<'_> {
    type Error = Infallible;
    fn base_tick(&mut self, session: &mut DreamSession, code: i32) -> Result<(), Self::Error> {
        self.trace.push(json!(["base_tick", code]));
        if let Some(state) = self.input["base_state"].as_i64() {
            session.state = state as i32;
        }
        Ok(())
    }
    fn base_notify(&mut self, _session: &mut DreamSession, code: i32) -> Result<(), Self::Error> {
        self.trace.push(json!(["base_notify", code]));
        Ok(())
    }
    fn sender_type(&self) -> u32 {
        self.input["sender_type"].as_u64().unwrap() as u32
    }
    fn start_day(&mut self) -> Result<i32, Self::Error> {
        self.trace.push(json!(["start_day"]));
        Ok(self.input["start_day"].as_i64().unwrap() as i32)
    }
    fn end_day(&mut self, mode: i32) -> Result<i32, Self::Error> {
        self.trace.push(json!(["end_day", mode]));
        Ok(self.input["end_day"].as_i64().unwrap() as i32)
    }
    fn cinematic(&mut self) -> Result<[i16; 4], Self::Error> {
        self.trace.push(json!(["cinematic"]));
        Ok([3, self.input["cinematic"].as_i64().unwrap() as i16, 7, 9])
    }
    fn current_map(&mut self) -> Result<i32, Self::Error> {
        Ok(self.input["current_map"].as_i64().unwrap() as i32)
    }
    fn stop_map(&mut self, source: Option<u32>) -> Result<(), Self::Error> {
        self.trace.push(if let Some(source) = source {
            json!(["stop_map_source", source])
        } else {
            json!(["stop_map"])
        });
        Ok(())
    }
    fn destroy_map(&mut self) -> Result<(), Self::Error> {
        self.trace.push(json!(["destroy_map"]));
        Ok(())
    }
    fn create_map(&mut self, stage: i32) -> Result<(), Self::Error> {
        self.trace.push(json!(["create_map", stage]));
        Ok(())
    }
    fn attach_map(&mut self) -> Result<(), Self::Error> {
        self.trace.push(json!(["attach_map"]));
        Ok(())
    }
    fn run_map(&mut self) -> Result<(), Self::Error> {
        self.trace.push(json!(["run_map"]));
        Ok(())
    }
    fn finish(&mut self, code: i32) -> Result<(), Self::Error> {
        self.trace.push(json!(["finish", code]));
        Ok(())
    }
}
#[test]
fn original_session_end_day_and_next_tick_rebuild_order_match() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/session-policy.json")).unwrap();
    for family in ["link_cases", "tick_cases", "route_cases"] {
        for case in fixture[family].as_array().unwrap() {
            let mut session = DreamSession {
                state: case["initial"]["state"].as_i64().unwrap() as i32,
                result: 17,
            };
            let mut host = Host {
                input: &case["initial"],
                trace: Vec::new(),
            };
            let code = case["event"].as_i64().unwrap() as i32;
            match family {
                "link_cases" => session.link_code(&mut host, 66, code).unwrap(),
                "tick_cases" => session.tick(&mut host, code).unwrap(),
                _ => session.notify(&mut host, 66, code).unwrap(),
            }
            let expected = &case["output"];
            assert_eq!(session.state, expected["state"].as_i64().unwrap() as i32);
            assert_eq!(session.result, expected["result"].as_i64().unwrap() as i32);
            assert_eq!(host.trace, *expected["trace"].as_array().unwrap());
        }
    }
    assert_eq!(fixture["link_cases"].as_array().unwrap().len(), 756);
    assert_eq!(fixture["tick_cases"].as_array().unwrap().len(), 252);
    assert_eq!(fixture["route_cases"].as_array().unwrap().len(), 40);
}
