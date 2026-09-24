# ADR 0021: Plan edits happen on the backend's stored plan, by id

## Status

Accepted

## Context

Find Duplicates picks which copy to keep automatically (oldest, or shortest path). Users need to overrule that: "keep _this_ one instead". That changes what the plan does. A file that was going to stay now goes to the trash, and one that was going to the trash stays. [ADR 0016](0016-backend-owned-plans.md) made the backend the owner of the plan, and the webview may only choose _which_ planned operations run. Any edit has to keep that property.

The plan also had no record of which files were duplicates of which. The keeper only appeared inside each copy's reason text.

## Decision

- A dedup plan carries its structure explicitly. `Plan.duplicate_sets` lists each set's `id`, `keeper` path and size, and the trash folder name. Each copy's `Operation.duplicate_set` names its set. Both fields are omitted for other modes.
- New command `choose_keeper(plan_id, operation_id)` → `choose_keeper_at(&PlanStore, …)`, per [ADR 0011](0011-command-layer-testing-via-extracted-functions.md). It edits the stored plan in place through `PlanStore::update`, then returns the edited plan for the preview:
  - The chosen copy's operation is removed, and that file becomes the set's keeper.
  - The old keeper becomes a new, selected `MoveToTrash` operation. It is staged exactly like the other copies (`staged_destination` with the set's trash folder name).
  - Every copy's reason is rewritten to name the new keeper, and the summary is recomputed.
- The webview sends only ids, never paths. Every path in an edited plan is one the scan found.
- A stale plan id, or an operation that isn't a duplicate copy, is an `invalid_plan` error. The stored plan is left untouched.
- The plan keeps its id across edits, so Apply works exactly as before.
- The preview groups a dedup plan by duplicate set ("Keeping x.jpg in C:\… — 2 copies"), and each copy has a **Keep this one** button. Other modes still group by destination folder.

## Consequences

- There is now a general pattern for plan edits: a command that takes ids and changes the stored plan through `PlanStore::update`. Future edits (e.g. changing one file's destination category) should follow it rather than accept paths from the webview.
- The frontend has to reconcile its selection after an edit. It keeps the choices for surviving ids, and the new operation starts selected.
- Every set always keeps exactly one file, because a swap only ever exchanges one copy for the keeper.
