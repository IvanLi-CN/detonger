import type { WebBlePrinterClient } from "../types";
import { BrowserWebBlePrinterClient } from "./client";
import { MockWebBlePrinterClient } from "./mock";

export function createPrinterClient(): WebBlePrinterClient {
  if (shouldUseMockBle()) {
    return new MockWebBlePrinterClient();
  }
  return new BrowserWebBlePrinterClient();
}

export function shouldUseMockBle(): boolean {
  if (typeof window === "undefined") {
    return false;
  }
  const query = new URLSearchParams(window.location.search);
  return query.get("mockBle") === "1" || import.meta.env.VITE_MOCK_BLE === "1";
}
