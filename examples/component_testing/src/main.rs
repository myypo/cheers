use std::sync::Arc;

use axum::{Router, extract::State, routing::get};
use cheers::{Rendered, components::Doctype, prelude::*};

#[derive(Clone, Debug, PartialEq, Eq)]
struct CrewMember {
    name: String,
    assignment: String,
    risk: RiskLevel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RiskLevel {
    Low,
    Watch,
    High,
}

impl RiskLevel {
    fn label(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Watch => "watch",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ShiftBriefing {
    mine: String,
    foreman: String,
    crew: Vec<CrewMember>,
}

impl ShiftBriefing {
    fn watch_count(&self) -> usize {
        self.crew
            .iter()
            .filter(|member| member.risk != RiskLevel::Low)
            .count()
    }
}

trait StaffTheMiningCrew: Send + Sync {
    fn briefing(&self) -> ShiftBriefing;
}

#[derive(Clone)]
struct Ctx {
    staffing: Arc<dyn StaffTheMiningCrew>,
}

struct InMemoryMineStaffing {
    briefing: ShiftBriefing,
}

impl InMemoryMineStaffing {
    fn new(briefing: ShiftBriefing) -> Self {
        Self { briefing }
    }
}

impl StaffTheMiningCrew for InMemoryMineStaffing {
    fn briefing(&self) -> ShiftBriefing {
        self.briefing.clone()
    }
}

#[derive(Cheers)]
#[id("title")]
struct MineShiftBriefing {
    briefing: ShiftBriefing,
}

impl Render for MineShiftBriefing {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        ids!(id, id_title);

        let watch_count = self.briefing.watch_count();

        html! {
            section id=id aria:labelledby=id_title {
                h2 id=id_title { "Mine Shift Briefing" }
                dl aria:label="Shift details" {
                    dt { "Mine" }
                    dd { (self.briefing.mine.clone()) }
                    dt { "Foreman" }
                    dd { (self.briefing.foreman.clone()) }
                    dt { "Crew on watch" }
                    dd { (watch_count) }
                }
                table aria:label="Crew assignments" {
                    caption { "Crew assignments" }
                    thead {
                        tr {
                            th scope="col" { "Dwarf" }
                            th scope="col" { "Assignment" }
                            th scope="col" { "Risk" }
                        }
                    }
                    tbody {
                        @for member in &self.briefing.crew {
                            tr {
                                th scope="row" { (member.name.clone()) }
                                td { (member.assignment.clone()) }
                                td { (member.risk.label()) }
                            }
                        }
                    }
                }
            }
        }
        .render_to(buffer);
    }
}

struct Page {
    briefing: ShiftBriefing,
}

impl Render for Page {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        html! {
            Doctype;
            html {
                head {
                    title { "Component Testing" }
                }
                body {
                    main {
                        h1 { "The Component Testing Mine" }
                        p { "This page renders a component from an injected staffing use case." }
                        MineShiftBriefing briefing=(self.briefing.clone());
                    }
                }
            }
        }
        .render_to(buffer);
    }
}

async fn page(ctx: State<Ctx>) -> Rendered<String> {
    Page {
        briefing: ctx.staffing.briefing(),
    }
    .render()
}

async fn briefing_component(ctx: State<Ctx>) -> Rendered<String> {
    html! {
        Doctype;
        html {
            head {
                title { "Briefing Component" }
            }
            body {
                MineShiftBriefing briefing=(ctx.staffing.briefing());
            }
        }
    }
    .render()
}

cheers::app!(Ctx);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let staffing = Arc::new(InMemoryMineStaffing::new(ShiftBriefing {
        mine: "Mithril Deep".to_owned(),
        foreman: "Thorin Ironmantle".to_owned(),
        crew: vec![
            CrewMember {
                name: "Balin Stonehelm".to_owned(),
                assignment: "Eastern winch".to_owned(),
                risk: RiskLevel::Low,
            },
            CrewMember {
                name: "Dori Emberpick".to_owned(),
                assignment: "Lower gallery supports".to_owned(),
                risk: RiskLevel::Watch,
            },
            CrewMember {
                name: "Nori Deepdelver".to_owned(),
                assignment: "Floodgate inspection".to_owned(),
                risk: RiskLevel::High,
            },
        ],
    }));
    let ctx = Ctx { staffing };

    let app = app(
        Router::new()
            .route("/", get(page))
            .route("/components/briefing", get(briefing_component)),
        cheers::router::Config::default(),
    )?
    .with_state(ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use thirtyfour::prelude::*;

    use super::*;

    struct ScriptedMineStaffing {
        briefing: ShiftBriefing,
    }

    impl ScriptedMineStaffing {
        fn new(briefing: ShiftBriefing) -> Self {
            Self { briefing }
        }
    }

    impl StaffTheMiningCrew for ScriptedMineStaffing {
        fn briefing(&self) -> ShiftBriefing {
            self.briefing.clone()
        }
    }

    fn scripted_ctx() -> Ctx {
        let staffing = Arc::new(ScriptedMineStaffing::new(ShiftBriefing {
            mine: "Mock Quartz Shaft".to_owned(),
            foreman: "Testa Gemfinder".to_owned(),
            crew: vec![
                CrewMember {
                    name: "Mockli Gemfinder".to_owned(),
                    assignment: "Gem QA face".to_owned(),
                    risk: RiskLevel::Watch,
                },
                CrewMember {
                    name: "Stubin Copperbeard".to_owned(),
                    assignment: "Harness inspection".to_owned(),
                    risk: RiskLevel::High,
                },
            ],
        }));

        Ctx { staffing }
    }

    #[test]
    fn briefing_component_can_be_rendered_in_isolation_without_a_browser() {
        let ctx = scripted_ctx();
        let html = MineShiftBriefing {
            briefing: ctx.staffing.briefing(),
        }
        .render()
        .into_inner();

        assert!(html.contains("aria-labelledby=\"mine_shift_briefing-title\""));
        assert!(html.contains("<h2 id=\"mine_shift_briefing-title\">Mine Shift Briefing</h2>"));
        assert!(html.contains("<dl aria-label=\"Shift details\">"));
        assert!(html.contains("<table aria-label=\"Crew assignments\">"));
        assert!(html.contains("Mock Quartz Shaft"));
        assert!(html.contains("Testa Gemfinder"));
        assert!(html.contains("Mockli Gemfinder"));
        assert!(html.contains("Stubin Copperbeard"));
    }

    #[tokio::test]
    async fn page_uses_the_injected_staffing_usecase() {
        let app = app(
            Router::new()
                .route("/", get(page))
                .route("/components/briefing", get(briefing_component)),
            cheers::router::Config::default(),
        )
        .expect("create test app")
        .with_state(scripted_ctx());

        let app = cheers::test::App::new(app)
            .await
            .expect("start browser app");

        app.run(|app| async move {
            app.goto(app.url("/")).await?;

            let h1 = app.find(By::Tag("h1")).await?;
            assert!(h1.text().await?.contains("The Component Testing Mine"));

            let section_selector = format!(
                "section[aria-labelledby='{}']",
                MineShiftBriefing::id_title()
            );
            app.find(By::Css(&section_selector)).await?;

            let mine = app
                .find(By::Css("dl[aria-label='Shift details'] dd:nth-of-type(1)"))
                .await?;
            assert_eq!(mine.text().await?, "Mock Quartz Shaft");

            let first_member = app
                .find(By::Css(
                    "table[aria-label='Crew assignments'] tbody tr:nth-of-type(1)",
                ))
                .await?;
            let first_member = first_member.text().await?;
            assert!(first_member.contains("Mockli Gemfinder"));
            assert!(first_member.contains("Gem QA face"));

            let second_member = app
                .find(By::Css(
                    "table[aria-label='Crew assignments'] tbody tr:nth-of-type(2)",
                ))
                .await?;
            let second_member = second_member.text().await?;
            assert!(second_member.contains("Stubin Copperbeard"));
            assert!(second_member.contains("high"));

            Ok(())
        })
        .await
        .expect("page should use the injected staffing usecase");
    }

    #[tokio::test]
    async fn briefing_component_route_uses_the_injected_staffing_usecase() {
        let app = app(
            Router::new()
                .route("/", get(page))
                .route("/components/briefing", get(briefing_component)),
            cheers::router::Config::default(),
        )
        .expect("create test app")
        .with_state(scripted_ctx());

        let app = cheers::test::App::new(app)
            .await
            .expect("start browser app");

        app.run(|app| async move {
            app.goto(app.url("/components/briefing")).await?;

            let h2 = app.find(By::Tag("h2")).await?;
            assert!(h2.text().await?.contains("Mine Shift Briefing"));

            let foreman = app
                .find(By::Css("dl[aria-label='Shift details'] dd:nth-of-type(2)"))
                .await?;
            assert_eq!(foreman.text().await?, "Testa Gemfinder");

            let second_member = app
                .find(By::Css(
                    "table[aria-label='Crew assignments'] tbody tr:nth-of-type(2)",
                ))
                .await?;
            assert!(second_member.text().await?.contains("Stubin Copperbeard"));

            Ok(())
        })
        .await
        .expect("component route should use the injected staffing usecase");
    }
}
