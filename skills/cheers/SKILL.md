---
name: cheers
description: "Use this skill when editing an existing cheers fullstack Rust app."
---

# Component model

A component is any Rust value implementing `Render`. `#[derive(Cheers)]` only generates helpers; it does not implement `Render`.

```rust
#[derive(Cheers)]
#[id("input")]
struct TodoRow {
    #[id]
    id: u64,
    #[signal]
    editing: bool,
    title: String,
}

impl Render for TodoRow {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        ids!(id, id_input);
        signals!(signal_editing);
        html! {
            tr id=id {
                td { (@&self.title) }
                td {
                    input id=id_input !bind(signal_editing);
                }
            }
        }
        .render_to(buffer);
    }
}
```

- Bind all generated names inside `render_to`: `ids!(...)`, `signals!(...)`, `form_names!(...)`.
- Use associated helpers outside the component: `TodoRow::id(7)`, `TodoRow::id_input(7)`, `TodoRow::signal_editing(7)`.
- Use generated ids for stable patch targets; use `scoped_signal!` for local UI state.
- Use `#[prop(default(...))]` with grouped invocation: `Card title="Welcome" [] { ... }` or `[author="myypo"]`.
- Use `#[form]`, `#[form(name: Type)]`, `#[form_derive(...)]`, and generated action types for forms.
- Extract repeated or complex markup into `Render` components.

# Template syntax

Rendering and props:

```rust
html! {
    p { "literal" (owned_or_copy) (@&borrowed) }
    UserCard user=(user.clone());
    Badge label=(@&label);
    Card title="Welcome" [author="myypo"] {
        p { "Child" }
    }
}
```

- `(expr)` moves while `(@&expr)` borrows. Prefer borrowing over cloning when the template only renders a value.

Attributes:

```rust
html! {
    button type="submit" { "Save" }
    section id=id aria:labelledby=heading_id "data-code"=(code) {}
    div class=(attribute! { "btn btn-" (@&kind) }) {}
}
```

- Static: `name="value"`; dynamic: `name=expr`, `name=(expr)`, `name=(@&expr)`.
- Use `aria:label` / `aria:labelledby` for dashed/namespaced attributes; quote arbitrary names.
- Text and attributes are escaped by default. Use `Raw` / `RawAttribute` only for trusted, sanitized content.

Control flow:

```rust
html! {
    @let count = self.items.len();
    @if count == 0 {
        p { "Empty" }
    } @else {
        p { "Items: " (count) }
    }
    @if let Some(name) = self.name {
        p { (name) }
    }
    @match self.status {
        Status::Open => p { "Open" }
        Status::Closed => p { "Closed" }
    }
    ul {
        @for item in &self.items {
            li { (@&item.name) }
        }
    }
}
```

Keep branches small and move complex computation out of templates. Use `@for` when the template wraps or transforms each item. If you already have `Vec<T>`, slices, arrays, or `Option<T>` where `T: Render`, render them directly with `(items)` or `(@&self.items)`. Use `@while` only for condition-based loops.

Datastar attributes:

```rust
html! {
    input !bind(name_signal);
    span !text(name_signal) {}
    div !show(open_signal) {}
    button !indicator(fetching_signal) {}
    div !signals(count: 5) !computed(total: { (price) " + " (tax) }) {}
    button !on:click((SaveUserAction { id })) { "Save" }
    details !attr("open": { (open_signal) " ? '' : null" }) {}
}
```

Use generated action structs in `!on:*`; do not hardcode generated URLs and signals. Datastar expressions are JavaScript fragments. Use signals for small client-visible values, patches for structural HTML.

Common attributes: `!bind` for two-way input binding, `!signals` for initial/local values, `!computed` for read-only derived values, `!text`/`!show`/`!attr`/`!class`/`!style` for reactive DOM state, `!indicator` for fetch state, `!init`/`!effect` for side effects, `!preserve_attr` and `!ignore_morph` for morphing edge cases, and `!on:event` for events. 
Use the cheers crate docs when additional information on datastar attributes is needed.

Suspense:

```rust
html! {
    @async {
        @let data = load_data().await;
        p { (data) }
    } @else {
        p { "Loading..." }
    }
}
```

Use `@async` for streamed initial rendering with accessible, layout-stable fallbacks. Use `EventReceiver` for long-lived updates after initial response.

# State and app wiring

Prefer use-case traits in `Arc<dyn Trait>` for app state and examples:

```rust
trait StaffTheMiningCrew: Send + Sync {
    fn briefing(&self) -> ShiftBriefing;
}
#[derive(Clone)]
struct Ctx {
    crew: Arc<dyn StaffTheMiningCrew>,
}
```

Keep traits use-case-specific, not generic `Backend`. Handlers should extract state/path/form data, call the use case, then render a component or return a patch/event.

Full pages are better rendered through a shared layout/base component with `Doctype`, `CssStylesheet`, and `Scripts`. Include `Scripts` on pages using actions, patches, signals, Datastar attributes, streams, or other Cheers client behavior; pure read-only pages do not need it.

Use `include_css!("./path.css")`, `include_svg_sprite! { ... }`, and `cheers::app!(Ctx);` following the app's existing organization. Build test routers with the generated `app(...)` helper.

# Dynamic behavior

Choose the smallest layer that works:

1. Normal navigation: anchor, form submit, redirect.
2. `#[action]` returning `PatchElements`: default for structural server-rendered HTML updates.
3. Generated signals or `scoped_signal!`: small client-local state/display values.
4. `EventReceiver`: multiple/coordinated events or long-lived server push.
5. `JsScript`: last resort.

Keep backend state authoritative; do not mirror broad backend state into signals by default.

# Actions, patches, signals, streams

Actions generate `...Action` types. `Path<_>` arguments become action fields/path segments. `Form<_>` or `#[form]` makes the generated action string include form content-type options; do not add those manually.

```rust
#[action(PUT)]
async fn update_user(Path(id): Path<u64>, Form(form): Form<UserForm>) -> PatchElements {
    PatchElements::new().element(UserRow::from_form(id, form))
}
html! {
    form !on:submit((UpdateUserAction { id: user.id })) {
        button type="submit" { "Save" }
    }
}
```

- `PatchElements`: when the rendered element has the target id, the default outer morph is enough. Add `.id(generated_id)` / `.selector(...)` only when targeting something other than the rendered element's own id or when one response must target multiple elements. Set `.mode(...)` only for non-default operations such as `Inner`, `Replace`, `Append`, `Prepend`, `Before`, `After`, or `Remove`.
- `PatchSignals`: `.set(signal, value)`, `.remove(signal)`, `.only_if_missing()` for small values already read by the page.
- `EventReceiver`: create `(tx, rx) = events()`, send `PatchElements`, `PatchSignals`, or `JsScript`, return `rx`; spawn only for updates continuing after handler return.

# Testing

Read `testing.md` when adding or changing tests.

# Validation

After editing Cheers templates, format changed files, for example:

```bash
cargo cheers fmt --rustfmt <edited-files-or-directories>
```

Before finishing, run the app's normal checks, such as relevant `cargo test`, `cargo clippy`, or project-specific validation.

# Pitfalls

- `#[derive(Cheers)]` does not implement `Render`.
- Include `Scripts` for actions, signals, patches, streams, and Datastar client behavior.
- Use generated `...Action` types and generated ids; do not hardcode generated action URLs or specific patch ids.
- Keep semantic HTML and ARIA correct.
- Use browser tests only when real client behavior matters.
