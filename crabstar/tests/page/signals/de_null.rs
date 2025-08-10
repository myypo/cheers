use crabstar::{Fragment, page};
use serde::{Deserialize, Serialize};
use typed_jinja::{Template, template};

#[page]
#[template(path = "empty.html")]
#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Subscription {
    plan: String,
    active: bool,
}

#[page]
#[template(path = "empty.html")]
#[derive(Default, Serialize, Deserialize, Debug, PartialEq)]
struct Profile {
    #[signal]
    subscription: Option<Subscription>,
}

#[test]
fn handles_null() {
    let json = r#"
        { "subscription": null }
    "#;

    let got: ProfileSignals = serde_json::from_str(json).unwrap();
    let want = Profile::signals().subscription(None);
    assert_eq!(got, want);
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
    assert_eq!(got, want);
}
