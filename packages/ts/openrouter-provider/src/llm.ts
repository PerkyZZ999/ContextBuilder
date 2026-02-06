/**
 * @module llm
 * LLM client wrapping the Vercel AI SDK + OpenRouter provider.
 * Handles retries, structured output, and token tracking.
 */
import { generateObject } from "ai";
import { createOpenRouter } from "@openrouter/ai-sdk-provider";
import type { z } from "zod";

import {
  PageSummaryOutputSchema,
  PageDescriptionOutputSchema,
  SkillMdOutputSchema,
  RulesOutputSchema,
  StyleOutputSchema,
  DoDontOutputSchema,
  LlmsTxtOutputSchema,
  LlmsFullTxtOutputSchema,
} from "./schemas";
import type { EnrichmentResult, EnrichmentTask, TaskType } from "./schemas";
import { SYSTEM_PROMPT, buildPrompt } from "./prompts";

const MAX_RETRIES = 3;
const RETRY_BASE_MS = 1000;

/**
 * Map task type to its structured output zod schema.
 */
function getOutputSchema(taskType: TaskType): z.ZodObject<z.ZodRawShape> {
  switch (taskType) {
    case "summarize_page":
      return PageSummaryOutputSchema;
    case "generate_description":
      return PageDescriptionOutputSchema;
    case "generate_skill_md":
      return SkillMdOutputSchema;
    case "generate_rules":
      return RulesOutputSchema;
    case "generate_style":
      return StyleOutputSchema;
    case "generate_do_dont":
      return DoDontOutputSchema;
    case "generate_llms_txt":
      return LlmsTxtOutputSchema;
    case "generate_llms_full_txt":
      return LlmsFullTxtOutputSchema;
  }
}

/**
 * Extract the text result from a structured output object.
 */
function extractText(taskType: TaskType, obj: Record<string, unknown>): string {
  if (taskType === "summarize_page") {
    return (obj as { summary: string }).summary;
  }
  if (taskType === "generate_description") {
    return (obj as { description: string }).description;
  }
  return (obj as { content: string }).content;
}

/**
 * Stderr logger for the bridge subprocess.
 * stdout is reserved for JSON-lines protocol.
 */
function log(level: string, event: string, data: Record<string, unknown> = {}): void {
  const entry = { ts: new Date().toISOString(), level, event, ...data };
  process.stderr.write(`${JSON.stringify(entry)}\n`);
}

/**
 * Run a single enrichment task against OpenRouter with retry logic.
 */
export async function runEnrichment(
  task: EnrichmentTask,
  modelId: string,
  apiKey: string,
): Promise<EnrichmentResult> {
  const openrouter = createOpenRouter({ apiKey });
  const prompt = buildPrompt(task);
  const schema = getOutputSchema(task.task_type);

  let lastError: Error | undefined;

  for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
    const startMs = Date.now();
    try {
      const result = await generateObject({
        model: openrouter(modelId),
        schema,
        system: SYSTEM_PROMPT,
        prompt,
      });

      const latencyMs = Date.now() - startMs;
      const tokensIn = result.usage?.inputTokens ?? 0;
      const tokensOut = result.usage?.outputTokens ?? 0;
      const text = extractText(task.task_type, result.object as Record<string, unknown>);

      log("info", "enrichment_complete", {
        task_type: task.task_type,
        model: modelId,
        tokens_in: tokensIn,
        tokens_out: tokensOut,
        latency_ms: latencyMs,
        attempt,
      });

      return {
        text,
        tokens_in: tokensIn,
        tokens_out: tokensOut,
        model: modelId,
        latency_ms: latencyMs,
      };
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err));
      const latencyMs = Date.now() - startMs;

      const isRetryable = isRetryableError(lastError);

      log("warn", "enrichment_retry", {
        task_type: task.task_type,
        model: modelId,
        attempt,
        error: lastError.message,
        retryable: isRetryable,
        latency_ms: latencyMs,
      });

      if (!isRetryable || attempt === MAX_RETRIES) {
        break;
      }

      const delayMs = RETRY_BASE_MS * 2 ** (attempt - 1);
      await Bun.sleep(delayMs);
    }
  }

  throw lastError ?? new Error("enrichment failed with no error captured");
}

/**
 * Check if an error is retryable (429 rate limit or 5xx server error).
 */
function isRetryableError(err: Error): boolean {
  const msg = err.message.toLowerCase();
  if (msg.includes("429") || msg.includes("rate limit")) return true;
  if (msg.includes("500") || msg.includes("502") || msg.includes("503") || msg.includes("504"))
    return true;
  if (msg.includes("timeout")) return true;
  return false;
}
