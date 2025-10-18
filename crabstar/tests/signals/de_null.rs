use askama::Template;

#[derive(Template, Debug, PartialEq, Clone)]
#[template(path = "empty.html")]
struct Subscription {
    #[signal]
    plan: String,
    #[signal]
    active: bool,
}

#[derive(Template, Default, Debug, PartialEq)]
#[template(path = "empty.html")]
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
