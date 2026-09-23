import { readFilePreview } from "./api/commands";

/** How many thumbnails can be generated at once. The grid can put dozens of
 * image tiles on screen in one go; decoding all of them at once would queue
 * dozens of full-image decodes in the backend ahead of anything newer. */
const MAX_IN_FLIGHT = 6;

/** Thumbnails kept after they've been shown, so scrolling back up (which
 * remounts virtualized tiles) doesn't regenerate them. */
const CACHE_LIMIT = 300;

const cache = new Map<string, Promise<string>>();
const waiting: (() => void)[] = [];
let inFlight = 0;

async function withSlot<T>(work: () => Promise<T>): Promise<T> {
  if (inFlight >= MAX_IN_FLIGHT) {
    await new Promise<void>((resolve) => waiting.push(resolve));
  }
  inFlight++;
  try {
    return await work();
  } finally {
    inFlight--;
    waiting.shift()?.();
  }
}

/** A thumbnail data URL for the image at `path`, shared between every tile
 * that shows the same file. Rejects if the file can't be previewed; the
 * failure isn't cached, so a later attempt can succeed. */
export function loadThumbnail(path: string): Promise<string> {
  const cached = cache.get(path);
  if (cached) {
    // Re-insert so the Map's insertion order tracks recent use.
    cache.delete(path);
    cache.set(path, cached);
    return cached;
  }
  const pending = withSlot(() => readFilePreview(path));
  cache.set(path, pending);
  pending.catch(() => cache.delete(path));
  while (cache.size > CACHE_LIMIT) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
  }
  return pending;
}

/** For tests only. */
export function resetThumbnailCache() {
  cache.clear();
}
