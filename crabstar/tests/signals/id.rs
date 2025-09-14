use std::fmt::Display;
use std::str::FromStr;

use crabstar::{Nested, signal};
use serde::{Deserialize, Serialize};

use crate::read_axum_body;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
struct Id(String);

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl FromStr for Id {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[signal(path = "empty.html")]
#[derive(PartialEq, Eq, Debug, Clone)]
struct Pet {
    #[react(id)]
    id: Id,
    #[react]
    name: String,
}

#[signal]
#[derive(PartialEq, Eq, Debug, Clone)]
struct Owner {
    #[react(id)]
    id: i32,
    #[react]
    pets: Nested<PetSignals>,
    ssn: i32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
struct Tag {
    value: String,
}

#[signal]
#[derive(PartialEq, Eq, Debug, Clone)]
struct Country {
    #[react(id)]
    id: i32,
    #[react]
    owners: Nested<OwnerSignals>,
    #[react]
    tags: Vec<Tag>,
}

#[signal]
#[derive(PartialEq, Eq, Debug, Clone)]
struct Page {
    #[react]
    name: String,
    #[react]
    countries: Nested<CountrySignals>,
}

#[tokio::test]
async fn works_with_vec_hierarchy() {
    let input = {
        let country_1 = Country::signals(1)
            .owners(vec![Owner::signals(2).pets(vec![
                Pet::signals("42").name("Meowser".to_owned()),
                Pet::signals("69").name("Woofie".to_owned()),
            ])])
            .tags(vec![
                Tag {
                    value: "ok".to_owned(),
                },
                Tag {
                    value: "go".to_owned(),
                },
            ]);
        let country_2 = Country::signals(2).owners(vec![
            Owner::signals(3).pets(vec![Pet::signals("100").name("Chirper".to_owned())]),
        ]);

        Page::signals()
            .name("Home".to_owned())
            .countries(vec![country_1, country_2])
    };

    let body = read_axum_body(input.clone()).await;

    assert_eq!(
        body,
        r#"{"name":"Home","countries":{"1":{"owners":{"2":{"pets":{"42":{"name":"Meowser"},"69":{"name":"Woofie"}}}},"tags":[{"value":"ok"},{"value":"go"}]},"2":{"owners":{"3":{"pets":{"100":{"name":"Chirper"}}}}}}}"#
    );

    let output: PageSignals = serde_json::from_str(&body).unwrap();
    assert_eq!(output, input);
}
