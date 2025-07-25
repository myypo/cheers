use typed_jinja::{Template, template};

#[template(path = "name_surname.html")]
struct NameSurname {
    name: String,
    surname: String,
}

#[test]
fn can_render_name_surname() {
    let name = "Crab".to_owned();
    let surname = "Rave".to_owned();
    let t = NameSurname {
        name: name.clone(),
        surname: surname.clone(),
    };

    let got = t.render().unwrap();
    let want = format!("{} {}", name, surname);

    assert_eq!(got, want);
}
