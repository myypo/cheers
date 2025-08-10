use crabstar::{Fragment, fragment};
use serde::{Deserialize, Serialize};
use typed_jinja::Template;

#[fragment(path = "empty.html")]
#[derive(Clone, Default, Debug, PartialEq, Deserialize, Serialize)]
struct User {
    #[signal]
    name: String,
    registered: bool,
    #[signal]
    achievements: Vec<Achievement>,
}

#[fragment(path = "empty.html")]
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
struct Achievement {
    name: String,
    points: Option<i32>,
}

#[fragment(path = "empty.html")]
#[derive(Debug, Default, PartialEq, Clone, Deserialize, Serialize)]
struct Header {
    tip: String,
    #[signal(granular)]
    user: Option<User>,
    #[signal]
    avatar: String,
}

#[test]
fn converts_nested_signals_to_json() {
    let user = User::signals().name("dude".to_owned()).achievements(vec![
        Achievement {
            name: "Fell for it".to_owned(),
            points: Some(15),
        },
        Achievement {
            name: "Again".to_owned(),
            points: None,
        },
    ]);
    let avatar = "dog".to_owned();

    let got = Header::signals()
        .user(Some(user.clone()))
        .avatar(avatar.clone());

    let want = r#"
        {
            "user": {
                "name": "dude",
                "achievements": [
                    {
                        "name": "Fell for it",
                        "points": 15
                    },
                    {
                        "name": "Again",
                        "points": null
                    }
                ]
            },
            "avatar": "dog"
        }
        "#;
    assert_eq!(got.clone(), serde_json::from_str(want).unwrap());

    assert_eq!(got.user.unwrap().unwrap(), user);
    assert_eq!(got.avatar.unwrap(), avatar);
}
