import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { dismissToast, pushToast, runToastAction, toasts } from "./toast";

describe("toast store", () => {
  it("pushToast adds a toast and dismissToast removes it by id", () => {
    toasts.set([]);
    pushToast("success", "Saved.", 999999); // long duration so the timer doesn't fire during the test
    const [toast] = get(toasts);
    expect(toast.kind).toBe("success");
    expect(toast.message).toBe("Saved.");

    dismissToast(toast.id);
    expect(get(toasts)).toEqual([]);
  });

  it("auto-dismisses after the given duration", () => {
    vi.useFakeTimers();
    toasts.set([]);
    pushToast("error", "Failed.", 4000);
    expect(get(toasts)).toHaveLength(1);

    vi.advanceTimersByTime(4000);
    expect(get(toasts)).toHaveLength(0);
    vi.useRealTimers();
  });

  it("assigns increasing ids so pushes never collide", () => {
    toasts.set([]);
    pushToast("info", "one", 999999);
    pushToast("info", "two", 999999);
    const [first, second] = get(toasts);
    expect(second.id).toBeGreaterThan(first.id);
  });

  it("runToastAction runs the action once and dismisses the toast", () => {
    toasts.set([]);
    const run = vi.fn();
    pushToast("error", "Out of date.", 999999, { label: "Scan again", run });
    const [toast] = get(toasts);
    expect(toast.action?.label).toBe("Scan again");

    runToastAction(toast.id);
    expect(run).toHaveBeenCalledTimes(1);
    expect(get(toasts)).toEqual([]);

    runToastAction(toast.id); // already gone: no-op
    expect(run).toHaveBeenCalledTimes(1);
  });
});
