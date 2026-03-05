const FALLBACK_TEXT = "Detonger Web BLE";

export async function renderTextToPngBytes(
  text: string,
  printWidthDots: number,
): Promise<Uint8Array> {
  if (typeof document === "undefined") {
    throw new Error("text rendering requires a browser document");
  }

  const canvas = document.createElement("canvas");
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    throw new Error("unable to create 2D canvas context");
  }

  const normalized = normalizeText(text);
  const lines = normalized.split("\n");
  const lineHeight = 34;
  const horizontalPadding = 16;
  const verticalPadding = 16;

  canvas.width = printWidthDots;
  canvas.height = Math.max(64, verticalPadding * 2 + lines.length * lineHeight);

  ctx.fillStyle = "#ffffff";
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  ctx.fillStyle = "#000000";
  ctx.font = "24px 'SF Mono', 'PingFang SC', monospace";
  ctx.textBaseline = "top";

  let y = verticalPadding;
  for (const line of lines) {
    ctx.fillText(line, horizontalPadding, y, canvas.width - horizontalPadding * 2);
    y += lineHeight;
  }

  const blob = await canvasToPngBlob(canvas);
  const bytes = await blob.arrayBuffer();
  return new Uint8Array(bytes);
}

function normalizeText(value: string): string {
  const trimmed = value.trimEnd();
  if (trimmed.length === 0) {
    return FALLBACK_TEXT;
  }
  return trimmed;
}

function canvasToPngBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error("failed to serialize canvas to PNG blob"));
        return;
      }
      resolve(blob);
    }, "image/png");
  });
}
