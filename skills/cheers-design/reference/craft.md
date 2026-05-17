# Craft

Build a confirmed design brief into production-quality Cheers code, then inspect and improve it.

## Gate

Do not start code edits until one is true:

- The user confirmed a `shape` brief for this task.
- The user supplied an already-confirmed brief.
- The user explicitly asked to skip shaping and proceed.

Before editing, read this file and any other relevant reference files (`harden`, `optimize`, `animate`, `polish`). If `../cheers/SKILL.md` exists and is not already loaded, read it as additional implementation context; otherwise use the Cheers implementation baseline in the parent `SKILL.md`.

Before editing, briefly state the confirmed brief status, register, chosen Datastar layer, and that optimistic UI is off.

## Implementation passes

### 1. Inspect conventions

Find the app's existing:

- shared layout/base page and whether it includes `Scripts`
- `Render` components, `#[derive(Cheers)]`, generated ids/forms/signals
- CSS inclusion pattern (`include_css!`, app stylesheet, tokens)
- routes, actions, state/use-case traits, tests
- existing empty/error/loading patterns

Follow local conventions over introducing a new pattern.

### 2. Define the server contract

For each user interaction, choose the smallest layer:

1. Anchor, form submit, redirect.
2. `#[action]` returning `PatchElements`.
3. `PatchSignals` for small display values already on the page.
4. `EventReceiver` for long-lived/coordinated backend updates.
5. `JsScript` for server-pushed imperative behavior when declarative Datastar/native CSS cannot express it.
6. `include_js_bundle!` for reusable static helpers only when the code justifies a bundle.

Make the contract explicit in code:

- generated `...Action` structs for `!on:*`
- `#[form]` and generated `self.form_names()` bindings for forms
- generated ids for patch targets
- backend validation and backend-confirmed success/error rendering

No optimistic UI. In-progress state belongs to `!indicator`, disabled attributes, pending copy, and `aria-busy`.

### 3. Build semantic components

Use Cheers component patterns:

```rust
#[derive(Cheers)]
#[id("title")]
#[id("status")]
#[form(title: String)]
struct ProjectEditor {
    #[id]
    id: u64,
    title: String,
    error: Option<String>,
}

impl Render for ProjectEditor {
    fn render_to(&self, buffer: &mut Buffer<Element>) {
        let ProjectEditorIds {
            id,
            id_title,
            id_status,
        } = self.ids();
        let ProjectEditorFormNames { form_title } = self.form_names();
        scoped_signal!(signal_saving: bool);

        html! {
            form
                id=id
                !on:submit((SaveProjectAction { id: self.id }))
                !indicator(signal_saving)
                !attr("aria-busy": { (signal_saving) " ? 'true' : null" })
            {
                label for=id_title { "Project title" }
                input id=id_title name=form_title value=(@&self.title) aria:describedby=id_status;
                @if let Some(error) = &self.error {
                    p id=id_status role="alert" { (@&error) }
                }
                button type="submit" !attr("disabled": signal_saving) { "Save" }
            }
        }
        .render_to(buffer);
    }
}
```

Handlers should call use-case code and return rendered state:

```rust
#[action(POST)]
async fn save_project(Path(id): Path<u64>, Form(form): Form<ProjectEditorForm>) -> PatchElements {
    match save_project_use_case(id, form.title).await {
        Ok(project) => PatchElements::new().element(ProjectEditor::from(project)),
        Err(error) => PatchElements::new().element(ProjectEditor::with_error(id, error)),
    }
}
```

### 4. Visual and responsive quality

- Use existing tokens or establish local CSS variables before one-off values.
- Product UI: restrained color, consistent controls, clear density, complete states.
- Brand UI: specific visual point of view, imagery when content calls for it, non-template composition.
- Avoid nested cards, side-stripe accents, gradient text, decorative glass, and modal-first flows.
- Use semantic headings/landmarks and visible labels.
- Make touch targets large enough, text wrap safely, and layout adapt structurally on mobile.

### 5. Motion and feedback

Use CSS and Datastar attributes first:

- `!indicator` for request pending state
- CSS transitions for hover/focus/reveal
- `@media (prefers-reduced-motion: reduce)` alternatives
- native `dialog`, `popover`, and View Transitions when appropriate

Do not add a framework animation dependency for a Cheers UI unless the project already uses it and the effect is justified.

### 6. Validate

After edits:

1. Format changed Cheers templates, e.g. `cargo cheers fmt --rustfmt <files>`.
2. Run targeted tests or checks. Use browser tests only when client behavior needs proof.
3. Inspect rendered output when practical: mobile, tablet/small laptop, desktop.
4. Do one critique-and-fix pass against:
   - confirmed brief
   - no optimistic UI
   - generated ids/actions/forms
   - semantics/a11y
   - loading/error/empty/success states
   - responsive behavior
   - visual polish and AI-slop avoidance

## Present

Summarize:

- files changed
- Datastar layer used
- states covered
- validation run
- any remaining limitations or follow-up risks
