import { PRINTER_SERVICE_UUID, PRINTER_WRITE_CHARACTERISTIC_UUID } from "./constants";
import { createPrintError, isPrintError } from "./errors";
import type { WebBlePrinterClient } from "../types";

const WRITE_DELAY_MS = 5;

export class BrowserWebBlePrinterClient implements WebBlePrinterClient {
  private device: BluetoothDevice | undefined;
  private characteristic: BluetoothRemoteGATTCharacteristic | undefined;
  private onDisconnect: (() => void) | undefined;

  async requestAndConnect(): Promise<void> {
    if (!isWebBluetoothSupported()) {
      throw createPrintError(
        "unsupported",
        "当前环境不支持 Web Bluetooth，请使用 Chrome 桌面版并在 HTTPS/localhost 下访问。",
      );
    }

    if (this.characteristic && this.device?.gatt?.connected) {
      return;
    }

    let device: BluetoothDevice;
    try {
      device = await navigator.bluetooth.requestDevice({
        acceptAllDevices: true,
        optionalServices: [PRINTER_SERVICE_UUID],
      });
    } catch (error) {
      throw createPrintError(
        "permission_denied",
        `蓝牙授权被拒绝或取消：${error instanceof Error ? error.message : String(error)}`,
      );
    }

    this.detachDevice(this.device);
    this.characteristic = undefined;

    device.addEventListener("gattserverdisconnected", this.handleDisconnected);

    try {
      const server = await device.gatt?.connect();
      if (!server) {
        throw createPrintError("device_not_found", "设备未建立 GATT 连接。请重试。");
      }

      const service = await server.getPrimaryService(PRINTER_SERVICE_UUID).catch(() => undefined);
      if (!service) {
        throw createPrintError(
          "service_not_found",
          `未找到目标服务 UUID: ${PRINTER_SERVICE_UUID}`,
        );
      }

      const characteristic = await service
        .getCharacteristic(PRINTER_WRITE_CHARACTERISTIC_UUID)
        .catch(() => undefined);
      if (!characteristic) {
        throw createPrintError(
          "char_not_found",
          `未找到写入特征 UUID: ${PRINTER_WRITE_CHARACTERISTIC_UUID}`,
        );
      }

      this.device = device;
      this.characteristic = characteristic;
    } catch (error) {
      this.detachDevice(device);
      this.device = undefined;
      this.characteristic = undefined;
      if (isPrintError(error)) {
        throw error;
      }
      throw createPrintError(
        "device_not_found",
        `连接打印机失败：${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  async printMessages(messages: Uint8Array[]): Promise<void> {
    if (!this.characteristic || !this.device?.gatt?.connected) {
      throw createPrintError("device_not_found", "尚未连接打印机。请先连接后再打印。");
    }

    for (const message of messages) {
      const payload = new Uint8Array(message).buffer;
      try {
        if (typeof this.characteristic.writeValueWithResponse === "function") {
          await this.characteristic.writeValueWithResponse(payload);
        } else {
          await this.characteristic.writeValue(payload);
        }
      } catch (error) {
        throw createPrintError(
          "write_failed",
          `写入 BLE 数据失败：${error instanceof Error ? error.message : String(error)}`,
        );
      }

      if (WRITE_DELAY_MS > 0) {
        await sleep(WRITE_DELAY_MS);
      }
    }
  }

  disconnect(): void {
    this.detachDevice(this.device, true);
    this.characteristic = undefined;
  }

  isConnected(): boolean {
    return Boolean(this.device?.gatt?.connected && this.characteristic);
  }

  setDisconnectHandler(handler: (() => void) | undefined): void {
    this.onDisconnect = handler;
  }

  private readonly handleDisconnected = () => {
    this.detachDevice(this.device);
    this.device = undefined;
    this.characteristic = undefined;
    this.onDisconnect?.();
  };

  private detachDevice(
    device: BluetoothDevice | undefined,
    waitForDisconnectEvent = false,
  ): void {
    if (!device) {
      return;
    }
    if (device.gatt?.connected) {
      device.gatt.disconnect();
      if (waitForDisconnectEvent) {
        return;
      }
    }
    device.removeEventListener("gattserverdisconnected", this.handleDisconnected);
  }
}

export function isWebBluetoothSupported(): boolean {
  return typeof navigator !== "undefined" && typeof navigator.bluetooth !== "undefined";
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
