/**
 * Shared by PreviewTable and BrowsePanel, which both group a flat list of
 * items by destination/source directory and track a multi-select `Record`
 * keyed by item id. Kept in one place after the two components' independent
 * copies of this logic diverged just enough to cause a shipped bug (the
 * grid-view volatile-each-block-key crash).
 */

export function dirOf(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx === -1 ? path : path.slice(0, idx);
}

export function fileNameOf(path: string): string {
  const idx = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return idx === -1 ? path : path.slice(idx + 1);
}

/** Groups `items` by directory (via `pathOf`), sorted by directory name. */
export function groupByDir<T>(
  items: T[],
  pathOf: (item: T) => string,
): [string, T[]][] {
  const map = new Map<string, T[]>();
  for (const item of items) {
    const key = dirOf(pathOf(item));
    const group = map.get(key);
    if (group) group.push(item);
    else map.set(key, [item]);
  }
  return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
}

/** Whether every item in `group` is selected in `selected`, keyed by `idOf`. */
export function isGroupChecked<T>(
  group: T[],
  idOf: (item: T) => string,
  selected: Record<string, boolean>,
): boolean {
  return group.length > 0 && group.every((item) => selected[idOf(item)]);
}

/** Whether some, but not all, of `group` is selected. */
export function isGroupIndeterminate<T>(
  group: T[],
  idOf: (item: T) => string,
  selected: Record<string, boolean>,
): boolean {
  return (
    !isGroupChecked(group, idOf, selected) &&
    group.some((item) => selected[idOf(item)])
  );
}

/** Returns a new selection record with every item in `group` set to `value`. */
export function withGroupSelection<T>(
  group: T[],
  idOf: (item: T) => string,
  selected: Record<string, boolean>,
  value: boolean,
): Record<string, boolean> {
  const next = { ...selected };
  for (const item of group) next[idOf(item)] = value;
  return next;
}
