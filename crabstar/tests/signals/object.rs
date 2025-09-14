use crabstar::signal;

#[signal]
#[derive(Clone, Default, Debug, PartialEq)]
struct User {
    #[react]
    name: String,
    registered: bool,
    #[react]
    achievements: Vec<AchievementSignals>,
}

#[signal]
#[derive(Debug, PartialEq, Clone)]
struct Achievement {
    #[react]
    name: String,
    #[react]
    points: Option<i32>,
}

#[signal]
#[derive(Debug, Default, PartialEq, Clone)]
struct Header {
    tip: String,
    #[react]
    user: Option<UserSignals>,
    #[react]
    avatar: String,
}

#[test]
fn converts_object_signals_to_json() {
    let user = User::signals().name("dude".to_owned()).achievements(vec![
        Achievement::signals()
            .name("Fell for it".to_owned())
            .points(Some(15)),
        Achievement::signals().name("Again".to_owned()).points(None),
    ]);
    let avatar = "dog".to_owned();

    let got = Header::signals().user(Some(user)).avatar(avatar.clone());

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
    let want: HeaderSignals = serde_json::from_str(want).unwrap();

    let got_user = got.user.unwrap().unwrap();
    let want_user = want.user.unwrap().unwrap();
    assert_eq!(got_user.name, want_user.name);
    assert_eq!(got_user.achievements, want_user.achievements);
    assert_eq!(got.avatar.unwrap(), avatar);
}
