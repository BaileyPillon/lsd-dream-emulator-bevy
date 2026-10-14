use lsd_core::{FramePhase, FrameSignal};
use serde_json::Value;
#[test]
fn original_clock_flags_counter_wrap_and_notification_codes_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/frame-phase.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut frame = FramePhase {
            phase: input[0].as_i64().unwrap() as i32,
            waiting: input[1].as_i64().unwrap() as i32,
            finished: input[2].as_i64().unwrap() as i32,
        };
        let signal = frame.advance();
        assert_eq!(frame.phase, case["phase"].as_i64().unwrap() as i32);
        assert_eq!(signal.code(), case["code"].as_i64().unwrap() as i32);
        frame.reset(-17);
        assert_eq!(
            frame,
            FramePhase {
                phase: -17,
                waiting: 0,
                finished: 0
            }
        );
        assert_eq!(frame.advance(), FrameSignal::Tick);
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 36);
}
