import { createInterface } from "node:readline";
import net from "node:net";

export type ScreenFormat = "text" | "json" | "json_compact";

export type Color =
  | { type: "Default" }
  | { type: "Indexed"; value: number }
  | { type: "Rgb"; value: [number, number, number] };

export interface CellAttributes {
  bold: boolean;
  italic: boolean;
  underline: boolean;
  inverse: boolean;
}

export interface ScreenCell {
  char: string;
  fg: Color;
  bg: Color;
  attrs: CellAttributes;
}

export interface ScreenJson {
  size: { cols: number; rows: number };
  cursor: { row: number; col: number };
  cells: ScreenCell[][];
}

export interface ScreenshotOptions {
  font?: string;
  fontSize?: number;
  lineHeight?: number;
}

export interface WaitOptions {
  timeoutMs?: number;
}

export interface WaitIdleOptions extends WaitOptions {
  idleMs: number;
}

type Pending = {
  resolve: (value: unknown) => void;
  reject: (reason?: unknown) => void;
};

type DaemonResponse = {
  id: number;
  result?: unknown;
  error?: { code: string; message: string; data?: unknown } | null;
};

export class TermwrightClient {
  private socket: net.Socket;
  private nextId = 1;
  private pending = new Map<number, Pending>();

  private constructor(socket: net.Socket) {
    this.socket = socket;
    const reader = createInterface({ input: socket });

    reader.on("line", (line) => {
      if (!line.trim()) return;
      let response: DaemonResponse;
      try {
        response = JSON.parse(line) as DaemonResponse;
      } catch (error) {
        this.rejectAll(error);
        return;
      }

      const pending = this.pending.get(response.id);
      if (!pending) return;
      this.pending.delete(response.id);

      if (response.error) {
        pending.reject(
          new Error(`${response.error.code}: ${response.error.message}`)
        );
        return;
      }

      pending.resolve(response.result);
    });

    socket.on("error", (error) => this.rejectAll(error));
    socket.on("close", () => this.rejectAll(new Error("socket closed")));
  }

  static async connect(socketPath: string): Promise<TermwrightClient> {
    const socket = net.createConnection({ path: socketPath });
    await new Promise<void>((resolve, reject) => {
      socket.once("connect", () => resolve());
      socket.once("error", (err) => reject(err));
    });

    return new TermwrightClient(socket);
  }

  async handshake(): Promise<{ protocol_version: number; termwright_version: string; pid: number }> {
    return (await this.call("handshake", null)) as {
      protocol_version: number;
      termwright_version: string;
      pid: number;
    };
  }

  async screen(format: ScreenFormat = "text"): Promise<unknown> {
    return this.call("screen", { format });
  }

  async screenText(): Promise<string> {
    return (await this.screen("text")) as string;
  }

  async screenJson(): Promise<ScreenJson> {
    return (await this.screen("json")) as ScreenJson;
  }

  async waitForText(text: string, options: WaitOptions = {}): Promise<void> {
    await this.call("wait_for_text", { text, timeout_ms: options.timeoutMs });
  }

  async waitForPattern(
    pattern: string,
    options: WaitOptions = {}
  ): Promise<void> {
    await this.call("wait_for_pattern", {
      pattern,
      timeout_ms: options.timeoutMs,
    });
  }

  async waitForIdle(options: WaitIdleOptions): Promise<void> {
    await this.call("wait_for_idle", {
      idle_ms: options.idleMs,
      timeout_ms: options.timeoutMs,
    });
  }

  async press(key: string): Promise<void> {
    await this.call("press", { key });
  }

  async type(text: string): Promise<void> {
    await this.call("type", { text });
  }

  async hotkey(options: { ctrl?: boolean; alt?: boolean; ch: string }): Promise<void> {
    const ch = options.ch.charAt(0);
    await this.call("hotkey", {
      ctrl: options.ctrl ?? false,
      alt: options.alt ?? false,
      ch,
    });
  }

  async screenshot(options: ScreenshotOptions = {}): Promise<Buffer> {
    const result = (await this.call("screenshot", {
      font: options.font,
      font_size: options.fontSize,
      line_height: options.lineHeight,
    })) as { png_base64: string };

    return Buffer.from(result.png_base64, "base64");
  }

  async close(): Promise<void> {
    try {
      await this.call("close", null);
    } finally {
      this.socket.end();
    }
  }

  private async call(method: string, params: unknown): Promise<unknown> {
    const id = this.nextId++;
    const payload = JSON.stringify({ id, method, params });

    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.socket.write(`${payload}\n`, (error) => {
        if (error) {
          this.pending.delete(id);
          reject(error);
        }
      });
    });
  }

  private rejectAll(error: unknown): void {
    for (const pending of this.pending.values()) {
      pending.reject(error);
    }
    this.pending.clear();
  }
}
