import { getRequestDeviceOptionsForConnect } from "../../src/ble/client";
import { PRINTER_SERVICE_UUID } from "../../src/ble/constants";

describe("getRequestDeviceOptionsForConnect", () => {
  it("enables name-prefix filtering by default", () => {
    const options = getRequestDeviceOptionsForConnect();
    expect(options).toEqual({
      filters: [{ namePrefix: "P2" }, { namePrefix: "Detonger" }],
      optionalServices: [PRINTER_SERVICE_UUID],
    });
  });

  it("allows showing all nearby devices when filtering is disabled", () => {
    const options = getRequestDeviceOptionsForConnect({
      filterDetongerDevices: false,
    });
    expect(options).toEqual({
      acceptAllDevices: true,
      optionalServices: [PRINTER_SERVICE_UUID],
    });
  });
});
