## Working style

The user acts as product owner and reviewer only: feature requests and review feedback come from them. Writing the implementation **and** its accompanying documentation is Claude's job, done proactively — not just when explicitly asked. The user's primary way of keeping up with the project is reading `CHANGELOG.md` and the docs below, not reading every diff.

Concretely, whenever a change is made to sortty:

- Update `CONTEXT.md` if the change introduces or changes a domain concept.
- Add an ADR under `docs/adr/` if the change reflects a real design decision (not just an implementation detail) — see the existing ADRs for tone and format.
- Add a `CHANGELOG.md` entry (newest first) summarizing what changed and why, in plain language.
- Update `README.md` if user-facing behavior, setup, or commands change.

Commit and push to `main` as part of finishing a unit of work, the same way tests are run before calling something done — don't wait to be asked. Use judgment on genuinely irreversible or destructive actions (force-push, history rewrites, deleting the repo) — those still warrant checking in first.

## Agent skills

### Issue tracker

Issues live in GitHub Issues (via `gh`) at [mikotorz/sortty](https://github.com/mikotorz/sortty). See `docs/agents/issue-tracker.md`.

### Domain docs

Single-context layout (`CONTEXT.md` + `docs/adr/` at repo root, created lazily as needed). See `docs/agents/domain.md`.
