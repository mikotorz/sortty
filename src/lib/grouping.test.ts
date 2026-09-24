import { describe, expect, it } from "vitest";
import {
  dirOf,
  fileNameOf,
  groupByDir,
  groupByDuplicateSet,
  isGroupChecked,
  isGroupIndeterminate,
  withGroupSelection,
} from "./grouping";

describe("dirOf / fileNameOf", () => {
  it("splits a Windows path", () => {
    expect(dirOf("C:\\Downloads\\Images\\a.png")).toBe("C:\\Downloads\\Images");
    expect(fileNameOf("C:\\Downloads\\Images\\a.png")).toBe("a.png");
  });

  it("splits a POSIX path", () => {
    expect(dirOf("/home/me/a.png")).toBe("/home/me");
    expect(fileNameOf("/home/me/a.png")).toBe("a.png");
  });

  it("returns the whole string when there's no separator", () => {
    expect(dirOf("a.png")).toBe("a.png");
    expect(fileNameOf("a.png")).toBe("a.png");
  });
});

describe("groupByDir", () => {
  it("groups by directory and sorts groups by directory name", () => {
    const items = ["b/2.txt", "a/1.txt", "b/1.txt"];
    const groups = groupByDir(items, (i) => i);
    expect(groups.map(([dir]) => dir)).toEqual(["a", "b"]);
    expect(groups.find(([dir]) => dir === "b")?.[1]).toEqual([
      "b/2.txt",
      "b/1.txt",
    ]);
  });
});

describe("group selection helpers", () => {
  type Item = { id: string };
  const idOf = (i: Item) => i.id;
  const group: Item[] = [{ id: "a" }, { id: "b" }, { id: "c" }];

  it("isGroupChecked is true only when every item is selected", () => {
    expect(isGroupChecked(group, idOf, { a: true, b: true, c: true })).toBe(
      true,
    );
    expect(isGroupChecked(group, idOf, { a: true, b: true })).toBe(false);
    expect(isGroupChecked([], idOf, {})).toBe(false);
  });

  it("isGroupIndeterminate is true only for a partial selection", () => {
    expect(isGroupIndeterminate(group, idOf, { a: true })).toBe(true);
    expect(
      isGroupIndeterminate(group, idOf, { a: true, b: true, c: true }),
    ).toBe(false);
    expect(isGroupIndeterminate(group, idOf, {})).toBe(false);
  });

  it("withGroupSelection sets every item in the group without mutating the input", () => {
    const selected = { a: true };
    const next = withGroupSelection(group, idOf, selected, false);
    expect(next).toEqual({ a: false, b: false, c: false });
    expect(selected).toEqual({ a: true });
  });
});

describe("groupByDuplicateSet", () => {
  const sets = [{ id: "s1" }, { id: "s2" }, { id: "s3" }];

  it("groups copies under their set, in the sets' order", () => {
    const copies = [
      { id: "c", duplicate_set: "s2" },
      { id: "a", duplicate_set: "s1" },
      { id: "b", duplicate_set: "s2" },
    ];
    expect(groupByDuplicateSet(sets, copies)).toEqual([
      { set: { id: "s1" }, copies: [{ id: "a", duplicate_set: "s1" }] },
      {
        set: { id: "s2" },
        copies: [
          { id: "c", duplicate_set: "s2" },
          { id: "b", duplicate_set: "s2" },
        ],
      },
    ]);
  });

  it("leaves out sets with no copies and copies with no set", () => {
    const copies = [{ id: "x" }, { id: "y", duplicate_set: "s3" }];
    expect(groupByDuplicateSet(sets, copies)).toEqual([
      { set: { id: "s3" }, copies: [{ id: "y", duplicate_set: "s3" }] },
    ]);
  });
});
