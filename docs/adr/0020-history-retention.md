# ADR 0020: History keeps runs for 90 days, but never fewer than the newest 200

## Status

Accepted

## Context

Every run was kept forever as `runs/<id>.json`, plus a line in `runs/index.json`. The folder grew without bound, and History read and sorted the whole index on every load. Pruning has a real cost, though: once a run is deleted it can't be undone, and undo is the app's core promise ([ADR 0002](0002-moves-not-deletes.md)).

## Decision

- After a new run is saved (by Apply or by a Browse delete), `store::prune_runs` removes runs that are **both**:
  - outside the newest 200 runs (`MIN_KEPT_RUNS`), **and**
  - started more than `history.keep_days` days ago.
- `keep_days` is a Setting (History → "Keep runs for (days)"). The default is 90. `0` keeps everything, and the maximum is 3650.
- The 200-run floor is a fixed constant, not a setting. It guarantees that a burst of old-but-recent-to-the-user runs (e.g. someone who uses sortty twice a year) never disappears just because of age.
- A run that isn't fully undone is pruned like any other once both conditions hold. Undoing moves from months ago is rarely wanted, and a "keep until undone" rule would bring back unbounded growth.
- Pruning is best effort. It runs after the run is saved, and a failure is only logged. It never fails the Apply or delete that triggered it.
- The new `[history]` settings table is `#[serde(default)]`, so older `settings.toml` files load unchanged.

## Consequences

- Needing both conditions makes the rule conservative. Nothing is ever pruned until there are more than 200 runs, and then only runs older than the setting.
- A pruned run's undo is gone for good. The Settings hint says so.
- The staged files themselves (in `.sortty-trash` / `.sortty-archive`) are untouched. Pruning only forgets the record of how to put them back.
