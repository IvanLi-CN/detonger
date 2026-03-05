import { expect, test } from "playwright/test";

const PNG_1X1_BASE64 =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO2W2X8AAAAASUVORK5CYII=";

test("mock BLE flow: connect and print text", async ({ page }) => {
  await page.goto("/?mockBle=1");

  await expect(page.getByRole("heading", { name: "Detonger Web BLE 打印" })).toBeVisible();
  await page.getByTestId("connect-btn").click();

  await expect(page.locator(".status-connected")).toBeVisible();
  await page.getByTestId("print-text-btn").click();

  await expect(page.getByTestId("log-list")).toContainText("文本打印完成");
});

test("mock BLE flow: upload png and print", async ({ page }) => {
  await page.goto("/?mockBle=1");
  await page.getByTestId("connect-btn").click();
  await expect(page.locator(".status-connected")).toBeVisible();

  await page.getByTestId("png-input").setInputFiles({
    name: "tiny.png",
    mimeType: "image/png",
    buffer: Buffer.from(PNG_1X1_BASE64, "base64"),
  });

  await page.getByTestId("print-png-btn").click();
  await expect(page.getByTestId("log-list")).toContainText("PNG 打印完成");
});
