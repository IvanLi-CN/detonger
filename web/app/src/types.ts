export type PrinterSessionState =
  | "idle"
  | "requesting"
  | "connected"
  | "printing"
  | "error";

export type PaperType = "continuous" | "gap";

export interface EncodeOptions {
  threshold: number;
  xOffsetDots: number;
  printWidthDots: number;
  paperType: PaperType;
}

export type PrintErrorCode =
  | "permission_denied"
  | "device_not_found"
  | "service_not_found"
  | "char_not_found"
  | "write_failed"
  | "protocol_error"
  | "unsupported";

export interface PrintError extends Error {
  code: PrintErrorCode;
}

export interface ConnectedPrinterInfo {
  id: string;
  name?: string;
}

export interface ConnectPrinterOptions {
  filterDetongerDevices?: boolean;
}

export interface WebBlePrinterClient {
  requestAndConnect(options?: ConnectPrinterOptions): Promise<ConnectedPrinterInfo>;
  printMessages(messages: Uint8Array[]): Promise<void>;
  disconnect(): void;
  isConnected(): boolean;
  setDisconnectHandler(handler: (() => void) | undefined): void;
}
