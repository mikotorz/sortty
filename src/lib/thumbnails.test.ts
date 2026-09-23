import { beforeEach, describe, expect, it, vi } from "vitest";
import { readFilePreview } from "./api/commands";
import { loadThumbnail, resetThumbnailCache } from "./thumbnails";

vi.mock("./api/commands", () => ({ readFilePreview: vi.fn() }));
const mockedPreview = vi.mocked(readFilePreview);

beforeEach(() => {
  resetThumbnailCache();
  mockedPreview.mockReset();
});

describe("loadThumbnail", () => {
  it("requests each file once and shares the result", async () => {
    mockedPreview.mockResolvedValue("data:x");
    const [a, b] = await Promise.all([
      loadThumbnail("C:\\a.png"),
      loadThumbnail("C:\\a.png"),
    ]);
    expect(a).toBe("data:x");
    expect(b).toBe("data:x");
    expect(mockedPreview).toHaveBeenCalledTimes(1);
  });

  it("never runs more than six requests at once", async () => {
    let running = 0;
    let peak = 0;
    mockedPreview.mockImplementation(async () => {
      running++;
      peak = Math.max(peak, running);
      await new Promise((r) => setTimeout(r, 5));
      running--;
      return "data:x";
    });
    await Promise.all(
      Array.from({ length: 20 }, (_, i) => loadThumbnail(`C:\\${i}.png`)),
    );
    expect(peak).toBe(6);
    expect(mockedPreview).toHaveBeenCalledTimes(20);
  });

  it("doesn't cache failures", async () => {
    mockedPreview.mockRejectedValueOnce("locked");
    await expect(loadThumbnail("C:\\a.png")).rejects.toBe("locked");
    mockedPreview.mockResolvedValueOnce("data:x");
    await expect(loadThumbnail("C:\\a.png")).resolves.toBe("data:x");
  });
});
