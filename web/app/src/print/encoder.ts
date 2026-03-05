import initWasm, {
  encodePngJobMessages,
  encodeWidthTestMessages,
  protocolVersion,
} from "../wasm/pkg/detonger_wasm";
import type { EncodeOptions } from "../types";

let initPromise: Promise<void> | undefined;

async function ensureWasmInitialized(): Promise<void> {
  if (!initPromise) {
    initPromise = initWasm().then(() => undefined);
  }
  return initPromise;
}

export async function encodePngMessages(
  pngBytes: Uint8Array,
  options: EncodeOptions,
): Promise<Uint8Array[]> {
  await ensureWasmInitialized();
  const messages = encodePngJobMessages(pngBytes, toWasmOptions(options));
  return normalizeMessages(messages);
}

export async function encodeWidthTestMessagesForPreview(
  options: EncodeOptions,
): Promise<Uint8Array[]> {
  await ensureWasmInitialized();
  const messages = encodeWidthTestMessages(toWasmOptions(options));
  return normalizeMessages(messages);
}

export async function getProtocolVersion(): Promise<string> {
  await ensureWasmInitialized();
  return protocolVersion();
}

function normalizeMessages(messages: Array<unknown>): Uint8Array[] {
  return messages.map((msg, index) => {
    if (!(msg instanceof Uint8Array)) {
      throw new Error(`unexpected wasm message type at index ${index}`);
    }
    return msg;
  });
}

function toWasmOptions(options: EncodeOptions): {
  threshold: number;
  xOffsetDots: number;
  printWidthDots: number;
} {
  return {
    threshold: options.threshold,
    xOffsetDots: options.xOffsetDots,
    printWidthDots: options.printWidthDots,
  };
}
