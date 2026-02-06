#!/usr/bin/env bun
/**
 * @module bridge
 * Enrichment bridge entrypoint — stdin/stdout JSON-lines protocol.
 *
 * Spawned by the Rust core as a subprocess:
 *   bun run packages/ts/openrouter-provider/src/bridge.ts
 *
 * Protocol:
 *   stdin  → JSON-lines: { type: "enrich", id, task } | { type: "shutdown" }
 *   stdout ← JSON-lines: { type: "result", id, result } | { type: "error", id, error } | { type: "ready" }
 *   stderr ← structured log lines (JSON)
 */
import { RequestMessageSchema } from "./schemas";
import type { ResponseMessage } from "./schemas";
import { runEnrichment } from "./llm";

// ---------------------------------------------------------------------------
// Config from environment
// ---------------------------------------------------------------------------

const API_KEY = process.env.OPENROUTER_API_KEY ?? "";
const MODEL_ID = process.env.CONTEXTBUILDER_MODEL ?? "moonshotai/kimi-k2.5";

function logStderr(level: string, event: string, data: Record<string, unknown> = {}): void {
  const entry = { ts: new Date().toISOString(), level, event, ...data };
  process.stderr.write(`${JSON.stringify(entry)}\n`);
}

/**
 * Send a response message to stdout (JSON-line).
 */
function send(msg: ResponseMessage): void {
  process.stdout.write(`${JSON.stringify(msg)}\n`);
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  if (!API_KEY) {
    logStderr("error", "missing_api_key", {
      message: "OPENROUTER_API_KEY environment variable is not set",
    });
    process.exit(1);
  }

  logStderr("info", "bridge_starting", { model: MODEL_ID });

  // Signal readiness
  send({ type: "ready" });

  // Read stdin line by line
  const reader = Bun.stdin.stream().getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();

      if (done) {
        logStderr("info", "stdin_closed");
        break;
      }

      buffer += decoder.decode(value, { stream: true });

      // Process complete lines
      let newlineIdx: number;
      while ((newlineIdx = buffer.indexOf("\n")) !== -1) {
        const line = buffer.slice(0, newlineIdx).trim();
        buffer = buffer.slice(newlineIdx + 1);

        if (!line) continue;

        await processLine(line);
      }
    }
  } finally {
    reader.releaseLock();
  }

  logStderr("info", "bridge_exiting");
}

async function processLine(line: string): Promise<void> {
  let parsed: unknown;
  try {
    parsed = JSON.parse(line);
  } catch {
    logStderr("error", "invalid_json", { line: line.slice(0, 200) });
    return;
  }

  const validated = RequestMessageSchema.safeParse(parsed);
  if (!validated.success) {
    logStderr("error", "invalid_message", {
      errors: validated.error.issues.map((i) => i.message),
    });
    return;
  }

  const msg = validated.data;

  if (msg.type === "shutdown") {
    logStderr("info", "shutdown_received");
    process.exit(0);
  }

  if (msg.type === "enrich") {
    try {
      const result = await runEnrichment(msg.task, MODEL_ID, API_KEY);
      send({ type: "result", id: msg.id, result });
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      logStderr("error", "enrichment_failed", {
        id: msg.id,
        task_type: msg.task.task_type,
        error: errorMsg,
      });
      send({ type: "error", id: msg.id, error: errorMsg });
    }
  }
}

main().catch((err) => {
  logStderr("fatal", "bridge_crash", { error: String(err) });
  process.exit(1);
});
