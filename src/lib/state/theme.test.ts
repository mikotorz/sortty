import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";

// theme.ts reads localStorage and subscribes to apply document.documentElement's
// data-theme attribute as soon as it's imported, so each test stubs globals
// first and re-imports a fresh module instance (vi.resetModules) rather than
// reusing the one shared singleton.
async function freshTheme() {
  vi.resetModules();
  return import("./theme");
}

describe("theme store", () => {
  it("defaults to system when localStorage is unavailable", async () => {
    vi.stubGlobal("localStorage", undefined);
    const { theme } = await freshTheme();
    expect(get(theme)).toBe("system");
    vi.unstubAllGlobals();
  });

  it("reads a previously stored theme", async () => {
    const store: Record<string, string> = { "sortty-theme": "dark" };
    vi.stubGlobal("localStorage", {
      getItem: (k: string) => store[k] ?? null,
      setItem: (k: string, v: string) => {
        store[k] = v;
      },
    });
    const { theme } = await freshTheme();
    expect(get(theme)).toBe("dark");
    vi.unstubAllGlobals();
  });

  it("ignores a garbage stored value and falls back to system", async () => {
    vi.stubGlobal("localStorage", {
      getItem: () => "not-a-real-theme",
      setItem: () => {},
    });
    const { theme } = await freshTheme();
    expect(get(theme)).toBe("system");
    vi.unstubAllGlobals();
  });

  it("applies data-theme to the document element, and clears it for system", async () => {
    const store: Record<string, string> = {};
    vi.stubGlobal("localStorage", {
      getItem: (k: string) => store[k] ?? null,
      setItem: (k: string, v: string) => {
        store[k] = v;
      },
    });
    const attrs: Record<string, string> = {};
    vi.stubGlobal("document", {
      documentElement: {
        setAttribute: (k: string, v: string) => {
          attrs[k] = v;
        },
        removeAttribute: (k: string) => {
          delete attrs[k];
        },
      },
    });
    const { theme } = await freshTheme();

    theme.set("dark");
    expect(attrs["data-theme"]).toBe("dark");
    expect(store["sortty-theme"]).toBe("dark");

    theme.set("system");
    expect(attrs["data-theme"]).toBeUndefined();
    expect(store["sortty-theme"]).toBe("system");

    vi.unstubAllGlobals();
  });
});
