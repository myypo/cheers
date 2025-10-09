use crabstar::crabstar;

#[crabstar(path = "empty.html", signal)]
#[derive(Debug, PartialEq, Clone)]
struct Subscription {
    #[signal]
    plan: String,
    #[signal]
    active: bool,
}

#[crabstar(path = "empty.html", signal)]
#[derive(Default, Debug, PartialEq)]
struct Profile {
    #[signal]
    subscription: Option<SubscriptionSignals>,
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
    let want = Profile::signals().subscription(Some(
        Subscription::signals()
            .plan("premium".to_owned())
            .active(true),
    ));
    assert_eq!(got.subscription, want.subscription);
}
