import { getContext, setContext } from "svelte";

/**
 * The app's page-level scroller (`<main class="app-content">`), shared with
 * virtualized lists so they scroll with the whole page instead of inside a
 * box of their own. `layoutVersion` is bumped whenever the page's content
 * resizes — anything that can shift a list's offset within the scroller
 * (a group toggling, the options card above growing) — so each list knows to
 * remeasure its `scrollMargin`. See ADR 0015.
 */
export interface ScrollRoot {
  el: HTMLElement | null;
  layoutVersion: number;
}

const KEY = Symbol("scrollRoot");

export function createScrollRoot(): ScrollRoot {
  const root = $state<ScrollRoot>({ el: null, layoutVersion: 0 });
  setContext(KEY, root);
  return root;
}

export function getScrollRoot(): ScrollRoot {
  return getContext<ScrollRoot>(KEY);
}
