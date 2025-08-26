use crabstar::{Signal, signal};
use serde::{Deserialize, Serialize};

#[signal]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Subscription {
    plan: String,
    active: bool,
}

#[signal]
#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
struct Profile {
    #[react]
    subscription: Option<Subscription>,
}

#[test]
fn handles_null() {
    let json = r#"
        { "subscription": null }
    "#;

    let got: ProfileSignals = serde_json::from_str(json).unwrap();
    let want = Profile::signals().subscription(None);
    assert_eq!(got.subscription, want.subscription);
}

#[test]
fn handles_specified_value() {
    let json = r#"
        { "subscription": { "plan": "premium", "active": true } }
    "#;

    let got: ProfileSignals = serde_json::from_str(json).unwrap();
    let want = Profile::signals().subscription(Some(Subscription {
        plan: "premium".to_string(),
        active: true,
    }));
    assert_eq!(got.subscription, want.subscription);
}
