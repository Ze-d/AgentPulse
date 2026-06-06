/**
 * AgentPulse frontend logger.
 *
 * Provides level-filtered console output and forwards ERROR/WARN messages
 * to the Rust backend via Tauri IPC for persistent file logging.
 *
 * Usage:
 *   import { createLogger } from "../utils/logger";
 *   const logger = createLogger("ModuleName");
 *   logger.info("something happened", extraData);
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type LogLevel = "ERROR" | "WARN" | "INFO" | "DEBUG" | "TRACE";

const LEVEL_ORDER: Record<LogLevel, number> = {
  ERROR: 0,
  WARN: 1,
  INFO: 2,
  DEBUG: 3,
  TRACE: 4,
};

/** Minimum level that gets forwarded to the Rust backend for persistence. */
const PERSIST_THRESHOLD: LogLevel = "WARN";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Check whether DEBUG/TRACE console output is enabled. */
function isDebugEnabled(): boolean {
  if (import.meta.env.DEV) {
    try {
      const params = new URLSearchParams(window.location.search);
      return params.has("debug");
    } catch {
      return false;
    }
  }
  return false;
}

function safeStringify(value: unknown): string {
  try {
    return JSON.stringify(value, null, 0);
  } catch {
    return String(value);
  }
}

function serializeArgs(args: unknown[]): string {
  return args
    .map((a) => {
      if (a instanceof Error) {
        return `${a.name}: ${a.message}\n${a.stack ?? ""}`;
      }
      if (typeof a === "object") {
        return safeStringify(a);
      }
      return String(a);
    })
    .join(" ");
}

// ---------------------------------------------------------------------------
// Logger
// ---------------------------------------------------------------------------

class AppLogger {
  private module: string;
  /** Lazily-resolved `invoke` function from Tauri. */
  private _invoke: (<T>(cmd: string, args?: Record<string, unknown>) => Promise<T>) | null = null;

  constructor(module: string) {
    this.module = module;
  }

  // -- public API --

  error(message: string, ...args: unknown[]): void {
    this.emit("ERROR", message, args);
  }

  warn(message: string, ...args: unknown[]): void {
    this.emit("WARN", message, args);
  }

  info(message: string, ...args: unknown[]): void {
    this.emit("INFO", message, args);
  }

  debug(message: string, ...args: unknown[]): void {
    this.emit("DEBUG", message, args);
  }

  trace(message: string, ...args: unknown[]): void {
    this.emit("TRACE", message, args);
  }

  // -- internal --

  private emit(level: LogLevel, message: string, args: unknown[]): void {
    this.writeConsole(level, message, args);
    this.maybeForward(level, message, args);
  }

  private writeConsole(level: LogLevel, message: string, args: unknown[]): void {
    const ts = new Date().toISOString();
    const prefix = `[${ts}] [${level}] [${this.module}]`;

    switch (level) {
      case "ERROR":
        console.error(prefix, message, ...args);
        break;
      case "WARN":
        console.warn(prefix, message, ...args);
        break;
      case "INFO":
        console.log(prefix, message, ...args);
        break;
      case "DEBUG":
      case "TRACE":
        if (isDebugEnabled()) {
          console.debug(prefix, message, ...args);
        }
        break;
    }
  }

  private maybeForward(level: LogLevel, message: string, args: unknown[]): void {
    // Only forward ERROR and WARN to the backend for persistent storage.
    if (LEVEL_ORDER[level] > LEVEL_ORDER[PERSIST_THRESHOLD]) {
      return;
    }

    this.forwardToBackend(level, message, args);
  }

  private async forwardToBackend(
    level: LogLevel,
    message: string,
    args: unknown[],
  ): Promise<void> {
    try {
      if (!this._invoke) {
        const tauriCore = await import("@tauri-apps/api/core");
        this._invoke = tauriCore.invoke;
      }

      const details = args.length > 0 ? serializeArgs(args) : null;

      await this._invoke("log_event", {
        level: level.toLowerCase(),
        module: this.module,
        message,
        details,
      });
    } catch {
      // Logging failures must never crash the app — silently drop.
    }
  }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

/**
 * Create a namespaced logger instance.
 *
 * @param module - Human-readable module name (e.g. "SessionStore", "FloatingPanel").
 */
export function createLogger(module: string): AppLogger {
  return new AppLogger(module);
}
