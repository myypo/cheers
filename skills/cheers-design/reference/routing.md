# Command guidance

## Workflow questions

Give advice without executing commands; the menu below is only for bare invocations. Consult the relevant command references for prerequisites and scope. If the user also requests execution, follow that request.

## No-argument routing: the context-aware menu

Read this when the user invokes `cheers-design` with no argument. They are asking "what should I do?" Make the menu context-aware instead of static. **Never auto-run a command; the recommendation is a suggestion the user confirms.**

Gather signals cheaply, in one pass:

- whether `PRODUCT.md` and `DESIGN.md` exist, and whether the project has UI code (templates, components, stylesheets);
- the latest critique snapshots: `ls .cheers-design/critique/` and, for the most relevant slug, `node <skill-dir>/scripts/critique-storage.mjs latest <slug>`;
- surface briefs in `.cheers-design/surfaces/`, especially any whose direction contract has no `FINISHED:` line;
- `git status --short`, to see which surfaces the dirty tree touches;
- whether a dev server is running, if the project's usual address is known.

With no PRODUCT.md, lead with `cheers-design init` as the top recommendation (one line on why) and still show the rest. Otherwise lead with the **2–3 highest-value next commands**, each with a one-line reason pulled from the signals, followed by the full Commands table from SKILL.md. Reason over the signals; there is no score to obey:

- No DESIGN.md while UI code exists → `document` (capture the incumbent system).
- No critique snapshot for a real surface → `critique <surface>` is a strong default.
- A latest critique with a low score or P0/P1 findings → `polish` on that surface (it reads the snapshot as its backlog). Snapshots are never closed, so when the target's files changed after the snapshot's timestamp (`git log -1 --format=%cI -- <files>` or the dirty tree), say the findings may be stale and suggest re-running `critique` instead.
- Changed files pointing at one surface → scope `audit` or `polish` to those files, naming them.
- A surface brief with an unfinished build → resume new-work's finish for that surface.
- Otherwise group by intent (build new / improve what's there), tailored to the current surface.

When the [mechanical scan](../SKILL.md#mechanical-scan-optional) is available, run it once: over the main page when a dev server is running, otherwise over the project's stylesheets. Fold the hits into your picks: many quality or contrast hits → `audit` or `polish`; a specific family → the matching command (gradient text or eyebrows → `quieter` or `typeset`, flat or gray palette → `colorize`). If it errors, is slow, or is unavailable, skip it and recommend the user run `audit` themselves; never block the suggestion on it.

Keep it to 2–3 pointed picks with the exact command to type. The menu stays the fallback; the recommendation is the lede.
