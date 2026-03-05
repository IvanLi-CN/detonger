import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import { createPrinterClient, shouldUseMockBle } from "./ble/factory";
import { toUserMessage } from "./ble/errors";
import { encodePngMessages, encodeWidthTestMessagesForPreview, getProtocolVersion } from "./print/encoder";
import { renderTextToPngBytes } from "./print/text-to-png";
import type { EncodeOptions, PrinterSessionState, WebBlePrinterClient } from "./types";

const DEFAULT_OPTIONS: EncodeOptions = {
  threshold: 150,
  xOffsetDots: 0,
  printWidthDots: 384,
};

type LogLevel = "info" | "error";

interface LogEntry {
  id: string;
  level: LogLevel;
  message: string;
  time: string;
}

function createLog(level: LogLevel, message: string): LogEntry {
  return {
    id: `${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    level,
    message,
    time: new Date().toLocaleTimeString(),
  };
}

function App() {
  const client = useMemo<WebBlePrinterClient>(() => createPrinterClient(), []);
  const [sessionState, setSessionState] = useState<PrinterSessionState>("idle");
  const [textInput, setTextInput] = useState("Detonger Web BLE\nHello from Chrome");
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [options, setOptions] = useState<EncodeOptions>(DEFAULT_OPTIONS);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [protocolVer, setProtocolVer] = useState<string>("loading...");
  const mockMode = shouldUseMockBle();
  const manualDisconnectRef = useRef(false);

  const appendLog = useCallback((level: LogLevel, message: string): void => {
    setLogs((prev) => [createLog(level, message), ...prev].slice(0, 60));
  }, []);

  useEffect(() => {
    client.setDisconnectHandler(() => {
      setSessionState("idle");
      if (manualDisconnectRef.current) {
        manualDisconnectRef.current = false;
        appendLog("info", "已主动断开连接。");
        return;
      }
      appendLog("error", "打印机连接已断开，请重新连接。");
    });

    return () => {
      client.setDisconnectHandler(undefined);
      client.disconnect();
    };
  }, [appendLog, client]);

  useEffect(() => {
    void getProtocolVersion()
      .then((version) => setProtocolVer(version))
      .catch(() => setProtocolVer("unavailable"));
  }, []);

  const canPrint = client.isConnected() && sessionState !== "printing";

  async function handleConnect(): Promise<void> {
    setSessionState("requesting");
    appendLog("info", "正在请求蓝牙权限并连接设备...");

    try {
      await client.requestAndConnect();
      setSessionState("connected");
      appendLog("info", "连接成功，可开始打印。");
    } catch (error) {
      setSessionState("error");
      appendLog("error", toUserMessage(error));
    }
  }

  function handleDisconnect(): void {
    if (!client.isConnected()) {
      appendLog("info", "当前没有活动连接。");
      return;
    }
    manualDisconnectRef.current = true;
    client.disconnect();
    setSessionState("idle");
  }

  async function handlePrintText(): Promise<void> {
    if (!client.isConnected()) {
      appendLog("error", "尚未连接打印机。");
      setSessionState("error");
      return;
    }

    setSessionState("printing");
    appendLog("info", "开始文本转图并发送打印数据...");

    try {
      const pngBytes = await renderTextToPngBytes(textInput, options.printWidthDots);
      const messages = await encodePngMessages(pngBytes, options);
      await client.printMessages(messages);
      setSessionState("connected");
      appendLog("info", `文本打印完成（${messages.length} 条消息）。`);
    } catch (error) {
      setSessionState("error");
      appendLog("error", toUserMessage(error));
    }
  }

  async function handlePrintPng(): Promise<void> {
    if (!selectedFile) {
      appendLog("error", "请先选择 PNG 文件。");
      return;
    }
    if (!client.isConnected()) {
      appendLog("error", "尚未连接打印机。");
      return;
    }

    setSessionState("printing");
    appendLog("info", `开始打印 PNG：${selectedFile.name}`);

    try {
      const messages = mockMode
        ? [new Uint8Array([0x1f, 0x2b, 0x00, 0x01, 0xff])]
        : await (async () => {
            const buffer = await selectedFile.arrayBuffer();
            return encodePngMessages(new Uint8Array(buffer), options);
          })();
      await client.printMessages(messages);
      setSessionState("connected");
      appendLog("info", `PNG 打印完成（${messages.length} 条消息）。`);
    } catch (error) {
      setSessionState("error");
      appendLog("error", toUserMessage(error));
    }
  }

  async function handleWidthTest(): Promise<void> {
    if (!client.isConnected()) {
      appendLog("error", "尚未连接打印机。");
      return;
    }
    setSessionState("printing");
    appendLog("info", "开始发送 width-test 测试图...");
    try {
      const messages = await encodeWidthTestMessagesForPreview(options);
      await client.printMessages(messages);
      setSessionState("connected");
      appendLog("info", `width-test 打印完成（${messages.length} 条消息）。`);
    } catch (error) {
      setSessionState("error");
      appendLog("error", toUserMessage(error));
    }
  }

  return (
    <div className="page">
      <header className="card">
        <h1>Detonger Web BLE 打印</h1>
        <p>
          Chrome Desktop (macOS 优先) · Protocol v{protocolVer} · 当前状态：
          <span className={`status status-${sessionState}`}> {sessionState}</span>
        </p>
        <div className="row">
          <button
            type="button"
            data-testid="connect-btn"
            onClick={() => void handleConnect()}
            disabled={sessionState === "requesting" || sessionState === "printing"}
          >
            连接打印机
          </button>
          <button
            type="button"
            data-testid="disconnect-btn"
            onClick={handleDisconnect}
            disabled={!client.isConnected() || sessionState === "printing"}
          >
            断开连接
          </button>
        </div>
      </header>

      <section className="card">
        <h2>打印参数</h2>
        <div className="grid">
          <label>
            Threshold
            <input
              type="number"
              min={0}
              max={255}
              value={options.threshold}
              onChange={(event) =>
                setOptions((prev) => ({
                  ...prev,
                  threshold: Number(event.target.value),
                }))
              }
            />
          </label>
          <label>
            X Offset (dots)
            <input
              type="number"
              value={options.xOffsetDots}
              onChange={(event) =>
                setOptions((prev) => ({
                  ...prev,
                  xOffsetDots: Number(event.target.value),
                }))
              }
            />
          </label>
          <label>
            Print Width (dots)
            <input
              type="number"
              min={8}
              step={8}
              value={options.printWidthDots}
              onChange={(event) =>
                setOptions((prev) => ({
                  ...prev,
                  printWidthDots: Number(event.target.value),
                }))
              }
            />
          </label>
        </div>
      </section>

      <section className="card">
        <h2>文本打印</h2>
        <textarea
          data-testid="text-input"
          rows={5}
          value={textInput}
          onChange={(event) => setTextInput(event.target.value)}
        />
        <div className="row">
          <button
            type="button"
            data-testid="print-text-btn"
            disabled={!canPrint}
            onClick={() => void handlePrintText()}
          >
            打印文本
          </button>
          <button
            type="button"
            data-testid="print-width-test-btn"
            disabled={!canPrint}
            onClick={() => void handleWidthTest()}
          >
            打印 width-test
          </button>
        </div>
      </section>

      <section className="card">
        <h2>PNG 打印</h2>
        <input
          data-testid="png-input"
          type="file"
          accept="image/png"
          onChange={(event) => setSelectedFile(event.target.files?.[0] ?? null)}
        />
        <button
          type="button"
          data-testid="print-png-btn"
          disabled={!canPrint || !selectedFile}
          onClick={() => void handlePrintPng()}
        >
          打印 PNG
        </button>
      </section>

      <section className="card">
        <h2>运行日志</h2>
        <ul data-testid="log-list" className="log-list">
          {logs.length === 0 ? <li>[等待操作]</li> : null}
          {logs.map((entry) => (
            <li key={entry.id} className={`log-${entry.level}`}>
              [{entry.time}] {entry.message}
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

export default App;
