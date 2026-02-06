/**
 * @module prompts
 * System and user prompts for each enrichment task type.
 */
import type { EnrichmentTask, TaskType } from "./schemas";

const SYSTEM_PROMPT = `You are an expert technical writer and documentation analyst for ContextBuilder, a tool that converts documentation into AI-ready artifacts. You produce concise, accurate, well-structured output. Follow the requested format exactly.`;

/**
 * Build the user prompt for an enrichment task.
 */
export function buildPrompt(task: EnrichmentTask): string {
  switch (task.task_type) {
    case "summarize_page":
      return buildSummarizePage(task);
    case "generate_description":
      return buildGenerateDescription(task);
    case "generate_skill_md":
      return buildGenerateSkillMd(task);
    case "generate_rules":
      return buildGenerateRules(task);
    case "generate_style":
      return buildGenerateStyle(task);
    case "generate_do_dont":
      return buildGenerateDoDont(task);
    case "generate_llms_txt":
      return buildGenerateLlmsTxt(task);
    case "generate_llms_full_txt":
      return buildGenerateLlmsFullTxt(task);
  }
}

export { SYSTEM_PROMPT };

// ---------------------------------------------------------------------------
// Individual prompt builders
// ---------------------------------------------------------------------------

function buildSummarizePage(task: EnrichmentTask): string {
  return `Summarize the following documentation page in 1-3 concise sentences. Focus on what the page teaches or documents, not meta-commentary.

Title: ${task.title ?? "Untitled"}
Source: ${task.source_url ?? "unknown"}

---
${task.content ?? ""}
---

Provide a JSON object with a "summary" field.`;
}

function buildGenerateDescription(task: EnrichmentTask): string {
  return `Write a single-line description (max 120 characters) for this documentation page, suitable for an llms.txt entry. Be specific and informative.

Title: ${task.title ?? "Untitled"}
Source: ${task.source_url ?? "unknown"}

---
${task.content ?? ""}
---

Provide a JSON object with a "description" field.`;
}

function buildGenerateSkillMd(task: EnrichmentTask): string {
  return `Generate a SKILL.md file following the Agent Skills specification (https://agentskills.io/specification).

The SKILL.md should describe a knowledge base built from the documentation below. Include these sections:
- YAML frontmatter with: name, version (0.1.0), description, author, license (MIT)
- Overview: What this documentation covers
- Capabilities: What an AI agent can do with this knowledge
- Usage: How to reference this knowledge base
- Configuration: Key settings or concepts
- Examples: 2-3 practical usage examples

KB Name: ${task.kb_name ?? "Unknown"}
Source: ${task.kb_source_url ?? "unknown"}

Documentation TOC:
${task.toc_json ?? "[]"}

Page summaries:
${task.summaries_json ?? "{}"}

Provide a JSON object with a "content" field containing the full SKILL.md content.`;
}

function buildGenerateRules(task: EnrichmentTask): string {
  return `Extract coding rules and conventions from the following documentation. Present them as actionable directives that an AI coding agent should follow.

Format each rule as:
- A clear, imperative statement
- Brief rationale if needed

Group rules by category (e.g., Naming, Error Handling, Architecture, API Usage).

KB Name: ${task.kb_name ?? "Unknown"}
Source: ${task.kb_source_url ?? "unknown"}

Page summaries:
${task.summaries_json ?? "{}"}

Full documentation content:
${task.pages_json ?? ""}

Provide a JSON object with a "content" field containing the full rules.md content.`;
}

function buildGenerateStyle(task: EnrichmentTask): string {
  return `Extract API style and naming conventions from the following documentation. Focus on:
- Naming patterns (functions, variables, types, files)
- Code formatting preferences
- API design patterns
- Common idioms and conventions

KB Name: ${task.kb_name ?? "Unknown"}
Source: ${task.kb_source_url ?? "unknown"}

Page summaries:
${task.summaries_json ?? "{}"}

Full documentation content:
${task.pages_json ?? ""}

Provide a JSON object with a "content" field containing the full style.md content.`;
}

function buildGenerateDoDont(task: EnrichmentTask): string {
  return `Extract Do/Don't pairs from the following documentation. Each pair should highlight:
- DO: The correct approach with a brief code example if applicable
- DON'T: The incorrect approach and why it's wrong

Cover common pitfalls, best practices, and anti-patterns.

KB Name: ${task.kb_name ?? "Unknown"}
Source: ${task.kb_source_url ?? "unknown"}

Page summaries:
${task.summaries_json ?? "{}"}

Full documentation content:
${task.pages_json ?? ""}

Provide a JSON object with a "content" field containing the full do_dont.md content.`;
}

function buildGenerateLlmsTxt(task: EnrichmentTask): string {
  return `Generate an llms.txt file following the llmstxt.org format specification.

Format:
1. H1 title line: # <KB Name>
2. Blockquote summary (1-2 sentences)
3. Sections with entries: ## <Section Name>
4. Each entry: - [Page Title](url): One-line description

Use the TOC to organize sections and the summaries for descriptions.

KB Name: ${task.kb_name ?? "Unknown"}
Source: ${task.kb_source_url ?? "unknown"}

TOC:
${task.toc_json ?? "[]"}

Page summaries:
${task.summaries_json ?? "{}"}

Provide a JSON object with a "content" field containing the complete llms.txt file.`;
}

function buildGenerateLlmsFullTxt(task: EnrichmentTask): string {
  return `Generate an llms-full.txt file. This is a single concatenated Markdown document containing all documentation pages.

Format:
1. H1 title: # <KB Name> — Full Documentation
2. Table of contents linking to H2 sections
3. Each page as an H2 section with source URL as HTML comment
4. Full page content (preserve code blocks, tables, links)

KB Name: ${task.kb_name ?? "Unknown"}
Source: ${task.kb_source_url ?? "unknown"}

Pages (JSON array of {path, title, content}):
${task.pages_json ?? "[]"}

Provide a JSON object with a "content" field containing the complete llms-full.txt file.`;
}
