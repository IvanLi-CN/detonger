import type { WebBlePrinterClient } from "../types";
import { createPrintError } from "./errors";

export class MockWebBlePrinterClient implements WebBlePrinterClient {
  private connected = false;
  private onDisconnect: (() => void) | undefined;

  async requestAndConnect(): Promise<void> {
    this.connected = true;
  }

  async printMessages(messages: Uint8Array[]): Promise<void> {
    if (!this.connected) {
      throw createPrintError("device_not_found", "Mock BLE: not connected");
    }
    if (messages.length === 0) {
      throw createPrintError("protocol_error", "Mock BLE: empty message set");
    }
  }

  disconnect(): void {
    const wasConnected = this.connected;
    this.connected = false;
    if (wasConnected) {
      this.onDisconnect?.();
    }
  }

  isConnected(): boolean {
    return this.connected;
  }

  setDisconnectHandler(handler: (() => void) | undefined): void {
    this.onDisconnect = handler;
  }
}
