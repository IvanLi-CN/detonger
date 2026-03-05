import type { PrintError, PrintErrorCode } from "../types";

export function createPrintError(code: PrintErrorCode, message: string): PrintError {
  const error = new Error(message) as PrintError;
  error.code = code;
  return error;
}

export function toUserMessage(error: unknown): string {
  if (isPrintError(error)) {
    return `${error.code}: ${error.message}`;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

export function isPrintError(error: unknown): error is PrintError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof (error as { code?: unknown }).code === "string"
  );
}
