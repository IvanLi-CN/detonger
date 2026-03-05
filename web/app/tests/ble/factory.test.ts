import { shouldUseMockBle } from "../../src/ble/factory";

describe("shouldUseMockBle", () => {
  it("enables mock mode from querystring", () => {
    window.history.pushState({}, "", "/?mockBle=1");
    expect(shouldUseMockBle()).toBe(true);
  });

  it("disables mock mode without querystring", () => {
    window.history.pushState({}, "", "/");
    expect(shouldUseMockBle()).toBe(false);
  });
});
