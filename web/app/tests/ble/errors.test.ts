import { createPrintError, toUserMessage } from "../../src/ble/errors";

describe("toUserMessage", () => {
  it("formats structured print errors", () => {
    const error = createPrintError("write_failed", "unable to write");
    expect(toUserMessage(error)).toBe("write_failed: unable to write");
  });

  it("handles generic errors", () => {
    expect(toUserMessage(new Error("boom"))).toBe("boom");
  });
});
