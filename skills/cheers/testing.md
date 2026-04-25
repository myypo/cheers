# Testing Cheers apps

Prefer the smallest test layer that proves the behavior.

## Layers

1. **Render tests, no browser**: instantiate a component, call `.render().into_inner()`, assert important markup/text/accessibility. Use for pure `Render`, component shape, generated ids/forms, and static semantics.
2. **Handler/router tests with injected state**: build the app router with deterministic `.with_state(...)` doubles. Use when behavior depends on state, use cases, extractors, or routing.
3. **Browser tests with `cheers::test::App`**: use only for client behavior: actions, Datastar patches, signals, form submission, navigation, timing, or page-shell integration.

## Render tests

```rust
#[test]
fn component_renders_expected_markup() {
    let html = MineShiftBriefing {
        briefing: briefing_fixture(),
    }
    .render()
    .into_inner();
    assert!(html.contains("<h1>Mine shift briefing</h1>"));
    assert!(html.contains("aria-label=\"Shift details\""));
}
```

Assert contracts, not incidental whitespace or attribute order. Avoid snapshots unless exact serialization is the behavior under test.

## Injected state

Use the app's normal state shape with deterministic doubles. Generic state and `Arc<dyn Trait>` state both work; choose whichever matches the app architecture:

```rust
struct ScriptedCrew {
    briefing: ShiftBriefing,
}
impl StaffTheMiningCrew for ScriptedCrew {
    fn briefing(&self) -> ShiftBriefing {
        self.briefing.clone()
    }
}
```

## Browser app

```rust
#[tokio::test]
async fn page_uses_injected_state() -> Result<(), Box<dyn std::error::Error>> {
    let app = cheers::router::new(
        Router::new().route("/", get(page)),
        cheers::router::Config::default(),
    )?
    .with_state(test_ctx());
    let app = cheers::test::App::new(app).await?;

    app.run(|app| async move {
        app.goto(app.url("/")).await?;
        let heading = app.find(By::Tag("h1")).await?;
        assert!(heading.text().await?.contains("Mine shift briefing"));
        Ok(())
    })
    .await?;
    Ok(())
}
```

Build tests with `cheers::router::new(...)`. Register any generated actions on the Axum router with `.action::<SomeAction>()` before `.with_state(...)`. Include `Scripts` on pages that exercise actions, signals, patches, or streams.

## Actions and patches

Navigate, wait for the initial target, fill inputs by semantic selectors, click the real control, wait for the patched DOM, then assert final text/structure.

```rust
app.find(By::Css("input[name='name']")).await?.send_keys("Balin").await?;
app.query(By::Css("button")).with_text("Save").and_clickable().first().await?.click().await?;
let row = app.find(By::Css("tbody tr:last-child")).await?;
assert!(row.text().await?.contains("Balin"));
```

Prefer clicking user-facing controls over invoking generated action URLs directly.

## Isolated browser component routes

When browser behavior matters but the full page is noise, mount a focused route:

```rust
async fn component_route(ctx: State<Ctx>) -> Rendered<String> {
    html! { Doctype; html { body { MyComponent data=(ctx.usecase.load()); } } }.render()
}

Router::new().route("/", get(page)).route("/components/my-component", get(component_route))
```

## Selector preference

Prefer selectors in this order:

1. Semantic HTML/accessibility: headings, landmarks, labels, captions, row/column headers, `aria-label`, `aria-labelledby`.
2. Generated Cheers ids when stable ids are already part of the component.
3. CSS classes or app-owned attributes that are part of the app contract.
4. `data-testid` only when no meaningful user-facing hook exists.
5. XPath only when CSS cannot express the relationship cleanly or while debugging.

```rust
let section = format!("section[aria-labelledby='{}']", MyComponent::id_title());
app.find(By::Css(&section)).await?;

let row = app.find(By::Css("table[aria-label='Crew assignments'] tbody tr:nth-of-type(1)")).await?;
let text = row.text().await?;
assert!(text.contains("Mockli Gemfinder"));
assert!(text.contains("Gem QA face"));
```

Use exact text equality for narrow, stable text nodes. Avoid strict full-text matches for rows, cards, list items, or composite elements.

## Debugging WebDriver failures

Check that the route is mounted, test state has expected data, `Scripts` is present when client behavior is required, and assertions wait for a stable parent. Temporarily inspect `app.source().await?` if available. Prefer narrow selectors plus `contains` assertions over broad strict text waits.
