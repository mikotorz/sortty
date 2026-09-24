import { describe, expect, it } from "vitest";
import { errorKind, errorMessage } from "./errors";

describe("errorMessage / errorKind", () => {
  it("reads a structured backend error", () => {
    const e = { kind: "invalid_plan", message: "this preview is out of date" };
    expect(errorMessage(e)).toBe("this preview is out of date");
    expect(errorKind(e)).toBe("invalid_plan");
  });

  it("falls back for plain strings and Error objects", () => {
    expect(errorMessage("boom")).toBe("boom");
    expect(errorKind("boom")).toBeNull();
    expect(errorMessage(new Error("bad"))).toBe("bad");
    expect(errorKind(new Error("bad"))).toBeNull();
  });

  it("never renders [object Object] for unknown objects", () => {
    expect(errorKind({ foo: 1 })).toBeNull();
    expect(errorMessage(null)).toBe("null");
  });
});
