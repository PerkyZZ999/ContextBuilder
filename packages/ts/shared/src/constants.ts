/**
 * @module constants
 * Shared constants for ContextBuilder.
 */

/** Valid artifact file names. */
export const ARTIFACT_NAMES = [
  "llms.txt",
  "llms-full.txt",
  "SKILL.md",
  "rules.md",
  "style.md",
  "do_dont.md",
] as const;

export type ArtifactName = (typeof ARTIFACT_NAMES)[number];

/** Default configuration values (mirrors Rust `DefaultsConfig`). */
export const DEFAULT_CONFIG = {
  crawl_depth: 3,
  concurrency: 4,
  rate_limit_ms: 200,
  mode: "auto" as const,
  model: "moonshotai/kimi-k2.5",
  api_key_env: "OPENROUTER_API_KEY",
} as const;
