import { beforeEach, vi } from "vitest";

const { initMock, encodePngMock, encodeWidthMock, versionMock } = vi.hoisted(() => ({
  initMock: vi.fn(async () => undefined),
  encodePngMock: vi.fn(() => [new Uint8Array([0x1f, 0x2b])]),
  encodeWidthMock: vi.fn(() => [new Uint8Array([0x1f, 0x20])]),
  versionMock: vi.fn(() => "0.1.0"),
}));

vi.mock("../../src/wasm/pkg/detonger_wasm", () => ({
  default: initMock,
  encodePngJobMessages: encodePngMock,
  encodeWidthTestMessages: encodeWidthMock,
  protocolVersion: versionMock,
}));

import {
  encodePngMessages,
  encodeWidthTestMessagesForPreview,
  getProtocolVersion,
} from "../../src/print/encoder";

const options = {
  threshold: 150,
  xOffsetDots: 0,
  printWidthDots: 384,
  paperType: "gap",
};

describe("encoder boundary", () => {
  beforeEach(() => {
    initMock.mockClear();
    encodePngMock.mockClear();
    encodeWidthMock.mockClear();
    versionMock.mockClear();
  });

  it("encodes png messages via wasm", async () => {
    const result = await encodePngMessages(new Uint8Array([1, 2, 3]), options);
    expect(result).toHaveLength(1);
    expect(result[0]).toBeInstanceOf(Uint8Array);
    expect(encodePngMock).toHaveBeenCalledWith(
      new Uint8Array([1, 2, 3]),
      options,
    );
  });

  it("encodes width-test messages via wasm", async () => {
    const result = await encodeWidthTestMessagesForPreview(options);
    expect(result).toHaveLength(1);
    expect(result[0]).toBeInstanceOf(Uint8Array);
    expect(encodeWidthMock).toHaveBeenCalledWith(options);
  });

  it("exposes wasm protocol version", async () => {
    await expect(getProtocolVersion()).resolves.toBe("0.1.0");
    expect(versionMock).toHaveBeenCalledTimes(1);
  });
});
