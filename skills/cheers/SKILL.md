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
        let TodoRowIds { id, id_input } = self.ids();
        let TodoRowSignals { signal_editing } = self.signals();
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

- Destructure generated names inside `render_to`: `let TodoRowIds { ... } = self.ids();`, `let TodoRowSignals { ... } = self.signals();`, `let TodoRowFormNames { ... } = self.form_names();` instead of using field access like `ids.whatever`. If some of them end up being unused, remove the generating attribute instead of ignoring the unused values.
- Use associated helpers outside the component: `TodoRow::id(7)`, `TodoRow::id_input(7)`, `TodoRow::signal_editing(7)`.
- `#[signal]` is client-only by default and is not submitted with Datastar action payloads. Use `#[signal(global)]` only when a handler needs to receive that signal value.
- Use generated ids for stable patch targets; use `scoped_signal!` for ad-hoc client-only UI state inside a component method.
- Use `#[prop(default(...))]` with grouped invocation: `Card title="Welcome" [] { ... }` or `[author="myypo"]`.
- Use `#[form]`, `#[form(name: Type)]`, and `#[form_derive(...)]` for generated form types.
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
cheers::define_events! {
    emoji_click
}

scoped_signal!(signal_message: String);

html! {
    input !bind(name_signal);
    span !text(name_signal) {}
    div !show(open_signal) {}
    button !indicator(fetching_signal) {}
    div !signals(count: 5) !computed(total: { (price) " + " (tax) }) {}
    button !on:click((SaveUserAction { id })) { "Save" }
    button !on:click[prevent, debounce("250ms")]("save()") { "Save" }
    div !on_interval[duration("1s")]({ (count_signal) "++" }) {}
    textarea
        !bind(draft_signal)
        !on:focusout({ "localStorage.setItem('draft', " (draft_signal) ")" }) {}
    details !attr("open": { (open_signal) " ? '' : null" }) {}
    div !signals(signal_message: String::new()) {
        textarea !bind(signal_message) {}
        div !on:emoji_click({ (signal_message) " += evt.detail.unicode" }) { "emoji picker widget" }
    }
}
```

Use generated action structs in `!on:*`; do not hardcode generated URLs and signal paths. Register custom Datastar events with `cheers::define_events! { my_event }` before using `!on:my_event(...)`; Datastar expressions are JavaScript fragments. Datastar modifiers go before value parentheses, e.g. `!on:click[prevent]("...")` or `!on_interval[duration("1s")]("...")`; unquoted modifier names are checked against known plugin modifiers, while quoted names like `["future"]` opt out for custom/new modifiers.

Common attributes: `!bind` for two-way input binding, `!signals` for initial/local values, `!computed` for read-only derived values, `!text`/`!show`/`!attr`/`!class`/`!style` for reactive DOM state, `!indicator` for fetch state, `!init`/`!effect` for side effects, `!preserve_attr` and `!ignore_morph` for morphing edge cases, and `!on:event` for events. Use the cheers crate docs when additional Datastar attribute details are needed.

Use inline `{ ... }` fragments directly in Datastar attributes. Use `js!` only when the JavaScript fragment needs to be stored, reused, or passed around as a value:

```rust
let clear_draft = js! {
    "localStorage.removeItem('draft')"
};
html! {
    button !on:click(clear_draft) { "Discard draft" }
}
```

In inline JS fragments and `js!`, string literals are raw JavaScript source, interpolated Rust strings render as JS string literals, and signals/action values render in `JsSource` context. Keep expressions small; prefer actions/patches/signals over large inline scripts.

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

Prefer use-case-oriented state, either generic over a concrete implementation or using `Arc<dyn Trait>` when dynamic dispatch is a better fit:

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

Use `include_css!("./path.css")` for global CSS, `include_svg_sprite! { ... }` for global SVG, and `const PATH_JS_BUNDLE: JsBundle = include_js_bundle!("./path.js")` for scoped optimized JS.

# Dynamic behavior

Choose the smallest layer that works:

1. Normal navigation: anchor, form submit, redirect.
2. `#[action]` returning `PatchElements`: default for structural server-rendered HTML updates.
3. Generated `#[signal]` values or `scoped_signal!`: small client-side state/display values.
4. `EventReceiver`: multiple/coordinated events or long-lived server push.
5. `JsScript`: last resort for dynamic/server-pushed imperative code.
6. Const JS bundle: reusable static client helpers when there is a lot of JS.

Keep backend state authoritative; do not mirror broad backend state into signals by default.

# Actions, patches, signals, streams

Actions generate `...Action` types. `Path<_>` arguments become action fields/path segments. `Form<_>` or `#[form]` makes the generated action string include form content-type options; do not add those manually.

When adding a field to a generated form, keep the `#[form(...)]` declaration, `self.form_names()` destructuring, input `name=...` attributes, and handler `Form<GeneratedForm>` type in sync.

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

- Use generated `...Action` types and generated ids; do not hardcode generated action URLs or specific patch ids.
- Keep semantic HTML and ARIA correct.
