---
name: cheers
description: "Use this skill when editing an existing cheers fullstack Rust app."
---

# Workflow

1. Inspect the existing page, component, action, subscription, and layout code first.
2. Preserve the app's current naming, composition, and wiring patterns unless the user asks for a redesign.
3. Prefer the smallest coherent change.
4. Validate after macro-heavy edits.

# Core authoring model

## Components are `Render`

A type becomes usable as a component by implementing `Render`.

`#[derive(Cheers)]` does **not** implement `Render`. It only generates helper APIs around a struct.

Use this as the default mental model when editing cheers code.

Minimal pattern:

```rust
use cheers::prelude::*;

struct Greeting<'a> {
    name: &'a str,
}

impl Render for Greeting<'_> {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        html! {
            p { "Hello, " (self.name) }
        }
        .render_to(buffer);
    }
}
```

## When to derive `Cheers`

Use `#[derive(Cheers)]` when the component needs generated helpers for:
- ids
- signals
- forms

Example:

```rust
use cheers::prelude::*;

#[derive(Cheers)]
#[id("input")]
struct TodoRow {
    #[id]
    id: u64,
    #[signal]
    done: bool,
    #[form]
    title: String,
}

impl Render for TodoRow {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        ids!(id, id_input);
        signals!(signal_done);
        form_names!(form_title);

        html! {
            label for=id_input {
                input id=id_input type="checkbox" !bind(signal_done) name=(form_title);
                span { (self.title) }
            }
        }
        .render_to(buffer);
    }
}
```

## Default props for components

Use `#[derive(Cheers)]` with `#[prop(default(...))]` when a component should have optional props in `html!`.

Rules:
- fields with `#[prop(default(...))]` are optional
- other non-`children` fields are required
- `children` stays special and still comes from the component body
- optional/defaulted prop overrides go in a grouped `[...]` section

Example:

```rust
#[derive(Cheers)]
struct Card<'a, R> {
    title: &'a str,
    #[prop(default("anonymous"))]
    author: &'a str,
    children: R,
}

impl<'a, R: Render> Render for Card<'a, R> {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        html! {
            article {
                h2 { (self.title) }
                p { (self.author) }
                div { (self.children) }
            }
        }
        .render_to(buffer);
    }
}
```

Usage:

```rust
html! {
    Card title="Welcome" [] {
        "Body"
    }

    Card title="Welcome" [author="myypo"] {
        "Body"
    }
}
```

Optional props are opt-in at the call site: use a trailing `[...]` group. Use `[]` when you want defaults without overriding any optional props. This syntax is mutually exclusive with `..` defaults syntax.

```rust
html! {
    Badge [];
    Badge [kind="warning"];
}
```

## Generated helpers: inside vs outside the component

Inside the component, bind generated helpers explicitly with:
- `ids!(...)`
- `signals!(...)`
- `form_names!(...)`

These helper-binding macros are intentionally exhaustive. If you derive a helper but never bind it, reconsider whether that helper should exist at all.

Outside the component, use the generated associated functions instead.

Examples:

```rust
let row_id = TodoRow::id(7);
let row_input_id = TodoRow::id_input(7);
let done_signal = TodoRow::signal_done(7);
```

Generated form names are mainly for use inside the component that owns the form markup.

## Ids and patch targeting

Use generated ids when you need stable references to DOM nodes, especially for patch targeting.

Patterns:
- `#[id]` on a field gives the component instance id
- `#[id("name")]` on the struct creates additional namespaced ids

Prefer component-generated ids over handwritten selector strings when patching specific elements.

## Signals

Use signals for:
- input binding
- lightweight local UI interaction
- small client-visible values
- loading or visibility toggles

Do **not** use signals as the default place to mirror broad backend state if patching HTML is simpler.

Patterns:
- `#[signal]` on a field creates a signal accessor
- `#[signal(name: Type)]` on the struct creates an extra signal not backed by a field
- `#[signal(nested)]` nests another component's signal scope

Example:

```rust
#[derive(Cheers)]
struct Counter {
    #[signal]
    count: i32,
}

impl Render for Counter {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        signals!(signal_count);

        html! {
            span !text(signal_count) {}
        }
        .render_to(buffer);
    }
}
```

Outside the component, use the generated associated function:

```rust
let count = Counter::signal_count();
```

If the component has an id field, outside callers usually need that id:

```rust
let name = Project::signal_name(1);
```

## `scoped_signal!`

Use `scoped_signal!` only for component-local state.

Good fits:
- loading spinners
- local open/closed toggles
- temporary UI state that should not be addressed from outside the component

Example:

```rust
impl Render for Projects {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        scoped_signal!(signal_fetching: bool);

        html! {
            button !on:click("@get('/items')") !indicator(signal_fetching) { "Refresh" }
            div !show(signal_fetching) { "Loading..." }
        }
        .render_to(buffer);
    }
}
```

Do **not** treat scoped signals as stable external names. Their generated path includes component-instance and call-site-derived data.

## Forms

Use generated form helpers when a component owns named form fields.

Patterns:
- `#[form]` on a field includes that field in the generated `...Form` type
- `#[form(name: Type)]` on the struct adds an extra form field not backed by a struct field
- `#[form_derive(...)]` adds derives to the generated form type

Inside the component, bind names with `form_names!(...)`:

```rust
form_names!(form_title);

html! {
    input name=(form_title);
}
```

Form names are mainly component-local.

## Markup macros you will edit most often

- `html!` is the default macro for cheers markup.
- Use `(@&expr)` inside `html!` or `attribute!` when you need to borrow a captured value instead of moving it.
- `attribute!` builds a dynamic attribute value from multiple fragments.

Example:

```rust
let kind = String::from("primary");
let class = attribute! { "btn btn-" (kind) };

html! {
    button class=class { "Save" }
}
```

Common Datastar-style attribute patterns in cheers syntax:

```rust
html! {
    input !bind(name_signal);
    span !text(name_signal) {}
    div !show(open_signal) {}
    button !indicator(fetching_signal) {}
    div !signals(count: 5, label: "ok") {}
    div !computed(total: { (price) " + " (tax) }) {}
    button !on:click(save_action) {}
    button !on:click({ "console.log(" signal_name ")" }) {}
    div !attr("aria-expanded": { (open_signal) " ? 'true' : 'false'" }) {}
}
```

# Actions, patches, and streaming

## `#[action(METHOD)]`

Use `#[action(...)]` on async handler functions that should render to client-side Datastar action strings.

The macro generates a companion `...Action` type.

Example:

```rust
use axum::extract::Path;
use cheers::prelude::*;

#[action(POST)]
async fn save_user(Path(id): Path<u64>) {}

let action = SaveUserAction { id: 7 };
assert_eq!(action.render().into_inner(), "@post('/cheers/actions/save_user/7')");
```

## How action arguments map

`Path<_>` arguments become fields on the generated action struct and URL path segments in the rendered action string.

A handler becomes a form action when it takes either:
- `Form<_>`
- or an argument marked with `#[form]`

That causes the generated action string to include Datastar form content type options.

Example:

```rust
use axum::extract::Form;

#[action(PUT)]
async fn update(_: Form<MyForm>) {}

assert_eq!(
    UpdateAction {}.render().into_inner(),
    "@put('/cheers/actions/update',{contentType:'form'})"
);
```

## Default write flow: mutate on the server, then patch HTML

For a normal write flow in cheers:
1. the user triggers a `#[action(...)]`
2. the server updates backend state
3. the server returns `PatchElements`
4. Datastar patches the DOM

This is the default pattern to reach for.

## CQRS is the default scale-up path

The normal request/response flow above should stay the default for ordinary user-initiated writes.

Reach for the fuller CQRS-style split when that immediate response is not enough and the server needs to push coordinated updates to the client, such as notifications, background job progress, dashboards, or collaborative changes.

In cheers, that usually means:
- commands / writes: `#[action(...)]` mutates backend state and usually returns `PatchElements`
- reads: initial page rendering or long-lived `EventReceiver` streams deliver server-rendered UI updates
- backend state stays authoritative
- the same `Render` implementations can be reused for page loads, patches, and streamed updates
- signals are for small client-local concerns, not the main read model

## `PatchElements`

`PatchElements` targets DOM elements and sends rendered HTML back to the client.

Common builder methods:
- `PatchElements::new()`
- `.id(...)`
- `.selector(...)`
- `.mode(...)`
- `.use_view_transition()`
- `.element(...)`

Prefer `.id(...)` when targeting a specific component instance.

Example:

```rust
#[derive(Cheers)]
struct Row {
    #[id]
    id: u32,
    label: String,
}

impl Render for Row {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        ids!(id);

        html! {
            tr id=id { (self.label) }
        }
        .render_to(buffer);
    }
}

#[action(PATCH)]
async fn rename_row(Path(id): Path<u32>) -> PatchElements {
    let row = Row {
        id,
        label: "Updated".to_owned(),
    };

    PatchElements::new()
        .id(Row::id(id))
        .mode(PatchElementsMode::Outer)
        .element(row)
}
```

Use `.selector(...)` when the target is truly selector-based rather than component-instance-based.

Mode guidance:
- use `Outer` when replacing a whole rendered element
- use `Inner` when replacing a container's contents
- use `Append` / `Prepend` for list-like insertion
- use `Remove` for deletion

Calling `.element(...)` multiple times adds multiple rendered payloads to the same patch.

## `PatchSignals`

`PatchSignals` updates existing client-side signals without returning HTML.

Use `PatchSignals` when:
- many elements on the page already read from the same signal
- you are changing small state like counters, toggles or filters

Prefer `PatchElements` when:
- markup structure changes
- a whole component can be cleanly re-rendered
- server-rendered HTML is simpler than coordinating signal-driven DOM behavior

Builder methods:
- `PatchSignals::new()`
- `.set(signal, value)`
- `.remove(signal)`
- `.only_if_missing()`

Example:

```rust
use axum::extract::Path;
use cheers::prelude::*;

#[derive(Cheers)]
struct FaqPage<'a> {
    questions: &'a [(&'a str, &'a str)],
    #[signal]
    all_open: bool,
}

impl Render for FaqPage<'_> {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        signals!(signal_all_open);

        html! {
            section {
                h2 !text({ (signal_all_open) " ? 'FAQ (all open)' : 'FAQ (all closed)'" }) {}

                @for (question, answer) in self.questions {
                    details !attr("open": { (signal_all_open) " ? '' : null" }) {
                        summary !attr("aria-expanded": { (signal_all_open) " ? 'true' : 'false'" }) {
                            (question)
                        }
                        p { (answer) }
                    }
                }
            }
        }
        .render_to(buffer);
    }
}

#[action(PATCH)]
async fn set_faq_expanded(Path(all_open): Path<bool>) -> PatchSignals {
    PatchSignals::new().set(FaqPage::signal_all_open(), all_open)
}
```

This is a good fit when one server update should fan out through many signal-driven UI reads, such as an entire `@for` loop of disclosures switching between open and closed states together.

`PatchSignals::set(...)` is typed: the value should match the signal's Rust type.

Practical rule:
- if you want fine-grained server updates, model fine-grained signals
- if you find yourself wanting to partially patch large object-shaped signals, prefer introducing smaller leaf signals or return `PatchElements` instead

## `EventReceiver` and `events()`

Use `EventReceiver` for long-lived server-sent event streams or when one response needs to emit multiple UI updates.

Typical pattern:
1. create `(tx, rx)` with `events()`
2. send initial `PatchElements`, `PatchSignals`, or `JsScript` events through `tx`
3. return `rx` from the handler

You can usually send an initial burst of events before returning `rx` directly. Spawn a task only when updates need to continue after the handler returns.

Example:

```rust
use axum::http::StatusCode;
use cheers::prelude::*;

#[derive(Cheers)]
struct Status<'a> {
    #[id]
    id: u32,
    message: &'a str,
}

impl Render for Status<'_> {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        ids!(id);

        html! {
            p id=id { (self.message) }
        }
        .render_to(buffer);
    }
}

async fn subscribe() -> Result<EventReceiver, StatusCode> {
    let (tx, rx) = events();

    tx.send(
        PatchElements::new()
            .id(Status::id(1))
            .mode(PatchElementsMode::Outer)
            .element(Status {
                id: 1,
                message: "Connected",
            }),
    )
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    tx.send(JsScript::new("console.log('subscription ready')"))
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    Ok(rx)
}
```

## `JsScript`

Use `JsScript` only when patching HTML is not enough.

Prefer:
- `PatchElements`
- normal markup updates
- Datastar attributes

Reach for `JsScript` only when you truly need client-side JavaScript execution.

# Page shell and app wiring

Page-shell concerns usually live in a top-level layout or base component.

Example:

```rust
use cheers::{
    components::{CssStylesheet, Doctype, Scripts},
    prelude::*,
};

struct Base<T> {
    children: T,
}

impl<T: Render> Render for Base<T> {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        html! {
            Doctype;
            html {
                head {
                    CssStylesheet;
                }
                body {
                    main { (self.children) }
                    Scripts;
                }
            }
        }
        .render_to(buffer);
    }
}
```

Use:
- `Doctype` at the top of full-page responses
- `CssStylesheet` in the page shell, usually in `head`
- `Scripts` in pages that rely on cheers / Datastar client behavior such as actions, signals, patches, or streaming updates

Use `include_css!("./path.css")` to declare stylesheet input for the Cheers CSS bundler. Prefer module scope so the declaration is clearly linked into the binary. Likewise, use `include_svg_sprite! { ... }` as a declarative asset registration for the global SVG sprite sheet.

Use `cheers::app!(StateType);` to generate the `app(...)` function that wires cheers routes into the application.

When touching shell or app wiring:
1. keep `Doctype`, `CssStylesheet`, and `Scripts` in a layout/base component if the app already has one
2. do not move shell concerns into leaf components unless explicitly asked
3. preserve the app's existing routing and startup shape unless the task specifically changes it

## Datastar heuristics for cheers edits

The sections above cover the main data-flow choices. Beyond that, use these defaults:

- replacing or morphing a whole component is often the right answer; do not over-optimize into tiny client-managed diffs too early
- prefer loading indicators over optimistic UI unless the user explicitly asks otherwise
- prefer normal web navigation patterns like anchors and redirects unless the task needs more
- keep semantic HTML and ARIA correct; Datastar does not replace accessibility work

For realtime or progressive interaction, first choose between the simple request/response patch flow and the CQRS/SSE scale-up path above, then add only the smallest necessary client-side signals.

# Easy-to-miss pitfalls

- If an action path changes, update the handler signature instead of hardcoding a URL string.
- If an action already uses `Form<_>` or `#[form]`, do not manually add Datastar form content-type options.
- When patching one component instance, prefer generated ids over handwritten selector strings.
- Use `Raw` / `RawAttribute` only for trusted, already-sanitized content.

# Formatting

After macro-heavy edits, run to format the Rust code as well as cheers macros:

```bash
cargo cheers fmt --rustfmt <edited-files-or-directories>
```
