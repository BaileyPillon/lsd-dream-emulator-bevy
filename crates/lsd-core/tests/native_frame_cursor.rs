use lsd_core::{FramePhase, FrameSubscribers};
use serde_json::{Value, json};

#[test]
fn original_live_clock_cursor_and_mutating_callbacks_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/frame-cursor.json")).unwrap();
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let mut frame = FramePhase::default();
        let mut subscribers = FrameSubscribers::default();
        for subscriber in 0..4_u64 {
            subscribers.subscribe(subscriber);
        }
        assert_eq!(json!(subscribers.order()), case["initial"]);
        let action = &case["action"];
        for (step, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
            // Changes to flags inside a callback affect the following notification.
            let signal = frame.advance();
            let mut observed = Vec::new();
            subscribers
                .try_notify::<()>(|subscriber, list| {
                    observed.push(json!({
                        "subscriber": subscriber,
                        "code": signal.code(),
                        "next": list.next_pending(),
                    }));
                    if step == 0 && subscriber == action["trigger"].as_u64().unwrap() {
                        let target = action["target"].as_u64().unwrap_or(subscriber);
                        match action["kind"].as_str().unwrap() {
                            "remove" => {
                                list.remove(target);
                            }
                            "restart" => {
                                list.remove(target);
                                list.subscribe(target);
                            }
                            "subscribe" => list.subscribe(target),
                            "destroy_self" => {
                                list.remove(subscriber);
                            }
                            "reset" => {
                                frame.reset(-17);
                                list.end_notification();
                            }
                            "wait" => frame.waiting = 1,
                            "finish" => frame.finished = 1,
                            _ => unreachable!(),
                        }
                    }
                    Ok(())
                })
                .unwrap();
            assert_eq!(
                json!({
                    "phase": frame.phase,
                    "waiting": frame.waiting,
                    "finished": frame.finished,
                    "order": subscribers.order(),
                    "observed": observed,
                    "cursor_cleared": subscribers.next_pending().is_none(),
                }),
                *expected,
                "case {number} step {step}, action {action}"
            );
            assert_eq!(subscribers.len(), subscribers.order().len());
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 88);
}

#[test]
fn failed_host_notification_releases_the_cursor() {
    let mut subscribers = FrameSubscribers::default();
    subscribers.subscribe(1);
    subscribers.subscribe(2);
    let result = subscribers.try_notify(|subscriber, list| {
        assert_eq!(subscriber, 2);
        list.remove(1);
        Err("host fault")
    });
    assert_eq!(result, Err("host fault"));
    assert_eq!(subscribers.next_pending(), None);
    assert_eq!(subscribers.order(), [2]);
}
