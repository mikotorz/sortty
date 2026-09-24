# ADR 0016: The backend keeps the plan; Apply takes an id

## Status

Accepted

## Context

`generate_plan` returned a `Plan` to the webview, and `apply_plan` took a whole `Plan` back from the webview and moved every selected `source` to its `destination`. So the IPC boundary effectively said "move any path to any other path". The backend never checked that the plan it was applying was one it had produced. The webview is our own code, but it renders file names and other data from the filesystem. Keeping the most powerful command in the app (bulk moves) independent of what the webview sends is cheap defense in depth.

It was also wasteful. A plan for a folder with tens of thousands of files was serialized to the webview for the preview, then serialized all the way back just to be applied.

## Decision

- `generate_plan` stores the plan it builds in a managed `PlanStore` (`commands/plan.rs`), then returns it for the preview as before.
- `apply_plan(plan_id, selected_ids)` looks the plan up by id and takes it out of the store. It applies only operations from that stored plan. An unknown or stale id gets a clear "this preview is out of date — scan again, then apply" error, and nothing moves.
- The store holds **only the latest plan**. Only one preview is ever on screen (Sort & Clean), a new scan replaces the old one, and applying a plan consumes it so it can't run twice.
- The logic lives in `apply_plan_at(&PlanStore, …)`, following [ADR 0011](0011-command-layer-testing-via-extracted-functions.md).

## Consequences

- The webview can now only choose _which_ of the backend's planned operations run (`selected_ids`), not what they do.
- Plans live only in memory. After an app restart the preview on screen is gone anyway, because the scan session isn't persisted, so nothing is lost.
- Browse delete (`delete_files`) still builds its plan from paths the webview picked. That is inherent, since the user is choosing arbitrary files. Instead it validates that every path is inside the browsed folder and that the folder isn't protected ([ADR 0006](0006-browse-delete-reuses-move-to-trash.md) amendment).
- This makes future plan edits possible on the backend, such as choosing which duplicate to keep: a command can modify the stored plan by id. (Done in [ADR 0021](0021-editing-the-stored-plan.md): `choose_keeper` edits the stored plan through `PlanStore::update`, and the plan keeps its id.)
