/**
 * @contextbuilder/openrouter-provider
 * OpenRouter + Vercel AI SDK bridge for LLM enrichment (stdin/stdout JSON-lines).
 */
export type {
  EnrichmentTask,
  EnrichmentResult,
  RequestMessage,
  ResponseMessage,
  TaskType,
} from "./schemas";
export {
  TASK_TYPES,
  EnrichmentTaskSchema,
  RequestMessageSchema,
  ResponseMessageSchema,
  EnrichmentResultSchema,
} from "./schemas";
export { runEnrichment } from "./llm";
