//! LLM enrichment orchestrator.
//!
//! Spawns the TypeScript bridge subprocess, sends enrichment tasks
//! via JSON-lines stdin/stdout protocol, and caches results in storage.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use sha2::{Digest, Sha256};
use tracing::{error, info, instrument, warn};

use contextbuilder_shared::{ContextBuilderError, PageMeta, Result, Toc};
use contextbuilder_storage::Storage;

// ---------------------------------------------------------------------------
// Protocol types (mirroring the TS schemas)
// ---------------------------------------------------------------------------

/// Task types matching the TS bridge protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    SummarizePage,
    GenerateDescription,
    GenerateSkillMd,
    GenerateRules,
    GenerateStyle,
    GenerateDoDont,
    GenerateLlmsTxt,
    GenerateLlmsFullTxt,
}

impl TaskType {
    /// Storage key for the enrichment cache.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SummarizePage => "summarize_page",
            Self::GenerateDescription => "generate_description",
            Self::GenerateSkillMd => "generate_skill_md",
            Self::GenerateRules => "generate_rules",
            Self::GenerateStyle => "generate_style",
            Self::GenerateDoDont => "generate_do_dont",
            Self::GenerateLlmsTxt => "generate_llms_txt",
            Self::GenerateLlmsFullTxt => "generate_llms_full_txt",
        }
    }
}

/// An enrichment task to send to the bridge.
#[derive(Debug, Clone, serde::Serialize)]
pub struct EnrichmentTask {
    pub task_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toc_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summaries_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kb_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kb_source_url: Option<String>,
}

/// Request message sent to the bridge.
#[derive(Debug, serde::Serialize)]
#[serde(tag = "type")]
enum RequestMessage {
    #[serde(rename = "enrich")]
    Enrich { id: String, task: EnrichmentTask },
    #[serde(rename = "shutdown")]
    Shutdown,
}

/// Response message received from the bridge.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type")]
enum ResponseMessage {
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "result")]
    Result {
        id: String,
        result: BridgeResult,
    },
    #[serde(rename = "error")]
    Error {
        #[allow(dead_code)]
        id: String,
        error: String,
    },
}

/// Enrichment result from the bridge.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeResult {
    pub text: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub model: String,
    pub latency_ms: u64,
}

// ---------------------------------------------------------------------------
// Enrichment results
// ---------------------------------------------------------------------------

/// Aggregated enrichment results for an entire KB.
#[derive(Debug, Clone, Default)]
pub struct EnrichmentResults {
    /// Page summaries keyed by page path.
    pub summaries: HashMap<String, String>,
    /// Page descriptions keyed by page path.
    pub descriptions: HashMap<String, String>,
    /// KB-level artifact content.
    pub skill_md: Option<String>,
    pub rules: Option<String>,
    pub style: Option<String>,
    pub do_dont: Option<String>,
    pub llms_txt: Option<String>,
    pub llms_full_txt: Option<String>,
    /// Total token usage.
    pub total_tokens_in: u64,
    pub total_tokens_out: u64,
    /// Model used.
    pub model: String,
    /// Number of cache hits.
    pub cache_hits: usize,
    /// Number of cache misses (LLM calls made).
    pub cache_misses: usize,
}

// ---------------------------------------------------------------------------
// Orchestrator config
// ---------------------------------------------------------------------------

/// Configuration for the enrichment orchestrator.
#[derive(Debug, Clone)]
pub struct EnrichmentConfig {
    /// Bridge command (e.g., "bun").
    pub bridge_cmd: String,
    /// Bridge script path (e.g., "packages/ts/openrouter-provider/src/bridge.ts").
    pub bridge_script: String,
    /// Working directory for the bridge.
    pub working_dir: String,
    /// Model ID for OpenRouter.
    pub model_id: String,
    /// KB name for context.
    pub kb_name: String,
    /// KB source URL for context.
    pub kb_source_url: String,
}

// ---------------------------------------------------------------------------
// Bridge handle
// ---------------------------------------------------------------------------

/// Handle to the spawned TS bridge subprocess.
struct BridgeHandle {
    child: Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    request_counter: u64,
}

impl BridgeHandle {
    /// Spawn the bridge subprocess.
    fn spawn(config: &EnrichmentConfig) -> Result<Self> {
        info!(cmd = %config.bridge_cmd, script = %config.bridge_script, "spawning enrichment bridge");

        let mut child = Command::new(&config.bridge_cmd)
            .arg("run")
            .arg(&config.bridge_script)
            .current_dir(&config.working_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Bridge logs go to parent stderr
            .spawn()
            .map_err(|e| {
                ContextBuilderError::Enrichment(format!(
                    "failed to spawn bridge: {e}. Is `{}` installed?",
                    config.bridge_cmd
                ))
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ContextBuilderError::Enrichment("failed to capture bridge stdin".into())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            ContextBuilderError::Enrichment("failed to capture bridge stdout".into())
        })?;

        let reader = BufReader::new(stdout);

        let mut handle = Self {
            child,
            stdin,
            reader,
            request_counter: 0,
        };

        // Wait for ready signal
        handle.wait_for_ready()?;

        Ok(handle)
    }

    /// Wait for the bridge to send its "ready" message.
    fn wait_for_ready(&mut self) -> Result<()> {
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| ContextBuilderError::Enrichment(format!("bridge read error: {e}")))?;

        let msg: ResponseMessage = serde_json::from_str(line.trim()).map_err(|e| {
            ContextBuilderError::Enrichment(format!(
                "invalid bridge ready message: {e} (got: {line})"
            ))
        })?;

        match msg {
            ResponseMessage::Ready => {
                info!("bridge is ready");
                Ok(())
            }
            _ => Err(ContextBuilderError::Enrichment(format!(
                "expected ready message, got: {line}"
            ))),
        }
    }

    /// Send an enrichment task and wait for the response.
    fn send_task(&mut self, task: EnrichmentTask) -> Result<BridgeResult> {
        self.request_counter += 1;
        let id = format!("req-{}", self.request_counter);

        let request = RequestMessage::Enrich {
            id: id.clone(),
            task,
        };

        let json = serde_json::to_string(&request).map_err(|e| {
            ContextBuilderError::Enrichment(format!("failed to serialize request: {e}"))
        })?;

        // Send request
        writeln!(self.stdin, "{json}").map_err(|e| {
            ContextBuilderError::Enrichment(format!("failed to write to bridge stdin: {e}"))
        })?;
        self.stdin.flush().map_err(|e| {
            ContextBuilderError::Enrichment(format!("failed to flush bridge stdin: {e}"))
        })?;

        // Read response
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| ContextBuilderError::Enrichment(format!("bridge read error: {e}")))?;

        if line.is_empty() {
            return Err(ContextBuilderError::Enrichment(
                "bridge closed stdout unexpectedly".into(),
            ));
        }

        let msg: ResponseMessage = serde_json::from_str(line.trim()).map_err(|e| {
            ContextBuilderError::Enrichment(format!(
                "invalid bridge response: {e} (got: {})",
                &line[..line.len().min(200)]
            ))
        })?;

        match msg {
            ResponseMessage::Result {
                id: resp_id,
                result,
            } => {
                debug_assert_eq!(resp_id, id);
                Ok(result)
            }
            ResponseMessage::Error {
                id: _,
                error,
            } => Err(ContextBuilderError::Enrichment(error)),
            ResponseMessage::Ready => Err(ContextBuilderError::Enrichment(
                "unexpected ready message during enrichment".into(),
            )),
        }
    }

    /// Send shutdown and wait for the bridge to exit.
    fn shutdown(mut self) -> Result<()> {
        let json = serde_json::to_string(&RequestMessage::Shutdown).unwrap();
        let _ = writeln!(self.stdin, "{json}");
        let _ = self.stdin.flush();

        match self.child.wait() {
            Ok(status) => {
                info!(?status, "bridge exited");
                Ok(())
            }
            Err(e) => {
                warn!("bridge wait error: {e}");
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public orchestrator API
// ---------------------------------------------------------------------------

/// Compute a prompt hash for cache keying.
fn prompt_hash(content: &str, task_type: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.update(task_type.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Run the full enrichment pipeline.
///
/// 1. Spawn bridge
/// 2. Summarize each page (with cache)
/// 3. Generate descriptions (with cache)
/// 4. Generate KB-level artifacts
/// 5. Shutdown bridge
#[instrument(skip_all, fields(kb = %config.kb_name, pages = pages.len()))]
pub async fn run_enrichment(
    config: &EnrichmentConfig,
    pages: &[(PageMeta, String)], // (meta, markdown_content)
    toc: &Toc,
    storage: &Storage,
    progress: &dyn EnrichmentProgress,
) -> Result<EnrichmentResults> {
    let mut results = EnrichmentResults {
        model: config.model_id.clone(),
        ..Default::default()
    };

    let kb_id = pages
        .first()
        .map(|(m, _)| m.kb_id.as_str())
        .unwrap_or("unknown");
    let total_tasks = pages.len() * 2 + 4; // summaries + descriptions + 4 KB artifacts
    let mut completed = 0;

    // --- Spawn bridge ---
    progress.phase("Starting enrichment bridge");
    let mut bridge = BridgeHandle::spawn(config)?;

    // --- Phase 1: Summarize each page ---
    progress.phase("Summarizing pages");
    for (meta, content) in pages {
        completed += 1;
        progress.task_progress(completed, total_tasks, &format!("Summarizing: {}", meta.path));

        let hash = prompt_hash(content, "summarize_page");

        // Check cache
        if let Some(cached) = storage
            .get_enrichment_cache(kb_id, "summarize_page", &hash, &config.model_id)
            .await?
        {
            results.summaries.insert(meta.path.clone(), cached);
            results.cache_hits += 1;
            continue;
        }

        let task = EnrichmentTask {
            task_type: "summarize_page".into(),
            content: Some(truncate_content(content, 12_000)),
            title: meta.title.clone(),
            source_url: Some(meta.url.clone()),
            toc_json: None,
            summaries_json: None,
            pages_json: None,
            kb_name: Some(config.kb_name.clone()),
            kb_source_url: Some(config.kb_source_url.clone()),
        };

        match bridge.send_task(task) {
            Ok(result) => {
                results.total_tokens_in += result.tokens_in;
                results.total_tokens_out += result.tokens_out;
                results.cache_misses += 1;

                // Cache result
                let _ = storage
                    .set_enrichment_cache(
                        kb_id,
                        "summarize_page",
                        &hash,
                        &config.model_id,
                        &result.text,
                    )
                    .await;

                results.summaries.insert(meta.path.clone(), result.text);
            }
            Err(e) => {
                warn!(path = %meta.path, error = %e, "page summarization failed");
            }
        }
    }

    // --- Phase 2: Generate descriptions ---
    progress.phase("Generating descriptions");
    for (meta, content) in pages {
        completed += 1;
        progress.task_progress(completed, total_tasks, &format!("Describing: {}", meta.path));

        let hash = prompt_hash(content, "generate_description");

        if let Some(cached) = storage
            .get_enrichment_cache(kb_id, "generate_description", &hash, &config.model_id)
            .await?
        {
            results.descriptions.insert(meta.path.clone(), cached);
            results.cache_hits += 1;
            continue;
        }

        let task = EnrichmentTask {
            task_type: "generate_description".into(),
            content: Some(truncate_content(content, 8_000)),
            title: meta.title.clone(),
            source_url: Some(meta.url.clone()),
            toc_json: None,
            summaries_json: None,
            pages_json: None,
            kb_name: Some(config.kb_name.clone()),
            kb_source_url: Some(config.kb_source_url.clone()),
        };

        match bridge.send_task(task) {
            Ok(result) => {
                results.total_tokens_in += result.tokens_in;
                results.total_tokens_out += result.tokens_out;
                results.cache_misses += 1;

                let _ = storage
                    .set_enrichment_cache(
                        kb_id,
                        "generate_description",
                        &hash,
                        &config.model_id,
                        &result.text,
                    )
                    .await;

                results
                    .descriptions
                    .insert(meta.path.clone(), result.text);
            }
            Err(e) => {
                warn!(path = %meta.path, error = %e, "description generation failed");
            }
        }
    }

    // --- Phase 3: KB-level artifacts ---
    let summaries_json = serde_json::to_string(&results.summaries).unwrap_or_default();
    let toc_json = serde_json::to_string(toc).unwrap_or_default();

    // Build a truncated pages JSON for KB-level tasks
    let pages_for_context: Vec<serde_json::Value> = pages
        .iter()
        .map(|(meta, content)| {
            serde_json::json!({
                "path": meta.path,
                "title": meta.title,
                "content": truncate_content(content, 4_000),
            })
        })
        .collect();
    let pages_json = serde_json::to_string(&pages_for_context).unwrap_or_default();

    // Generate each KB-level artifact
    let kb_tasks: Vec<(TaskType, &str)> = vec![
        (TaskType::GenerateSkillMd, "generate_skill_md"),
        (TaskType::GenerateRules, "generate_rules"),
        (TaskType::GenerateStyle, "generate_style"),
        (TaskType::GenerateDoDont, "generate_do_dont"),
    ];

    for (task_type, task_type_str) in &kb_tasks {
        completed += 1;
        progress.task_progress(
            completed,
            total_tasks,
            &format!("Generating: {task_type_str}"),
        );

        let hash = prompt_hash(&summaries_json, task_type_str);

        if let Some(cached) = storage
            .get_enrichment_cache(kb_id, task_type_str, &hash, &config.model_id)
            .await?
        {
            set_kb_artifact(&mut results, *task_type, cached);
            results.cache_hits += 1;
            continue;
        }

        let task = EnrichmentTask {
            task_type: (*task_type_str).into(),
            content: None,
            title: None,
            source_url: None,
            toc_json: Some(toc_json.clone()),
            summaries_json: Some(summaries_json.clone()),
            pages_json: Some(pages_json.clone()),
            kb_name: Some(config.kb_name.clone()),
            kb_source_url: Some(config.kb_source_url.clone()),
        };

        match bridge.send_task(task) {
            Ok(result) => {
                results.total_tokens_in += result.tokens_in;
                results.total_tokens_out += result.tokens_out;
                results.cache_misses += 1;

                let _ = storage
                    .set_enrichment_cache(
                        kb_id,
                        task_type_str,
                        &hash,
                        &config.model_id,
                        &result.text,
                    )
                    .await;

                set_kb_artifact(&mut results, *task_type, result.text);
            }
            Err(e) => {
                error!(task = task_type_str, error = %e, "KB artifact generation failed");
            }
        }
    }

    // --- Shutdown bridge ---
    progress.phase("Shutting down enrichment bridge");
    bridge.shutdown()?;

    info!(
        cache_hits = results.cache_hits,
        cache_misses = results.cache_misses,
        tokens_in = results.total_tokens_in,
        tokens_out = results.total_tokens_out,
        "enrichment complete"
    );

    Ok(results)
}

/// Set a KB-level artifact in the results.
fn set_kb_artifact(results: &mut EnrichmentResults, task_type: TaskType, text: String) {
    match task_type {
        TaskType::GenerateSkillMd => results.skill_md = Some(text),
        TaskType::GenerateRules => results.rules = Some(text),
        TaskType::GenerateStyle => results.style = Some(text),
        TaskType::GenerateDoDont => results.do_dont = Some(text),
        TaskType::GenerateLlmsTxt => results.llms_txt = Some(text),
        TaskType::GenerateLlmsFullTxt => results.llms_full_txt = Some(text),
        _ => {}
    }
}

/// Truncate content to approximately `max_chars` characters.
fn truncate_content(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        content.to_string()
    } else {
        let truncated = &content[..max_chars];
        format!("{truncated}\n\n[... content truncated for LLM context window ...]")
    }
}

// ---------------------------------------------------------------------------
// Progress trait
// ---------------------------------------------------------------------------

/// Progress callback for enrichment operations.
pub trait EnrichmentProgress: Send + Sync {
    /// Called when entering a new phase.
    fn phase(&self, name: &str);
    /// Task-level progress within the current phase.
    fn task_progress(&self, current: usize, total: usize, detail: &str);
}

/// No-op enrichment progress.
pub struct SilentEnrichmentProgress;

impl EnrichmentProgress for SilentEnrichmentProgress {
    fn phase(&self, _name: &str) {}
    fn task_progress(&self, _current: usize, _total: usize, _detail: &str) {}
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_hash_deterministic() {
        let h1 = prompt_hash("hello world", "summarize_page");
        let h2 = prompt_hash("hello world", "summarize_page");
        assert_eq!(h1, h2);
    }

    #[test]
    fn prompt_hash_differs_by_task() {
        let h1 = prompt_hash("hello", "summarize_page");
        let h2 = prompt_hash("hello", "generate_description");
        assert_ne!(h1, h2);
    }

    #[test]
    fn truncate_short_content() {
        let content = "short text";
        assert_eq!(truncate_content(content, 100), "short text");
    }

    #[test]
    fn truncate_long_content() {
        let content = "a".repeat(200);
        let result = truncate_content(&content, 100);
        assert!(result.len() > 100);
        assert!(result.contains("truncated"));
    }

    #[test]
    fn task_type_as_str() {
        assert_eq!(TaskType::SummarizePage.as_str(), "summarize_page");
        assert_eq!(TaskType::GenerateSkillMd.as_str(), "generate_skill_md");
        assert_eq!(TaskType::GenerateDoDont.as_str(), "generate_do_dont");
    }

    #[test]
    fn request_message_serializes_correctly() {
        let msg = RequestMessage::Enrich {
            id: "req-1".into(),
            task: EnrichmentTask {
                task_type: "summarize_page".into(),
                content: Some("test".into()),
                title: None,
                source_url: None,
                toc_json: None,
                summaries_json: None,
                pages_json: None,
                kb_name: None,
                kb_source_url: None,
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"enrich"#));
        assert!(json.contains(r#""id":"req-1"#));
        assert!(json.contains(r#""task_type":"summarize_page"#));
    }

    #[test]
    fn shutdown_message_serializes_correctly() {
        let msg = RequestMessage::Shutdown;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"shutdown"}"#);
    }

    #[test]
    fn response_message_deserializes_ready() {
        let json = r#"{"type":"ready"}"#;
        let msg: ResponseMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, ResponseMessage::Ready));
    }

    #[test]
    fn response_message_deserializes_result() {
        let json = r#"{"type":"result","id":"req-1","result":{"text":"summary","tokens_in":100,"tokens_out":50,"model":"test","latency_ms":200}}"#;
        let msg: ResponseMessage = serde_json::from_str(json).unwrap();
        match msg {
            ResponseMessage::Result { id, result } => {
                assert_eq!(id, "req-1");
                assert_eq!(result.text, "summary");
                assert_eq!(result.tokens_in, 100);
                assert_eq!(result.tokens_out, 50);
            }
            _ => panic!("expected Result"),
        }
    }

    #[test]
    fn response_message_deserializes_error() {
        let json = r#"{"type":"error","id":"req-2","error":"rate limited"}"#;
        let msg: ResponseMessage = serde_json::from_str(json).unwrap();
        match msg {
            ResponseMessage::Error { id, error } => {
                assert_eq!(id, "req-2");
                assert_eq!(error, "rate limited");
            }
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn enrichment_results_default() {
        let results = EnrichmentResults::default();
        assert!(results.summaries.is_empty());
        assert!(results.descriptions.is_empty());
        assert!(results.skill_md.is_none());
        assert_eq!(results.cache_hits, 0);
        assert_eq!(results.cache_misses, 0);
    }

    #[test]
    fn set_kb_artifact_works() {
        let mut results = EnrichmentResults::default();
        set_kb_artifact(&mut results, TaskType::GenerateSkillMd, "skill content".into());
        set_kb_artifact(&mut results, TaskType::GenerateRules, "rules content".into());
        set_kb_artifact(&mut results, TaskType::GenerateStyle, "style content".into());
        set_kb_artifact(&mut results, TaskType::GenerateDoDont, "dodont content".into());
        assert_eq!(results.skill_md.as_deref(), Some("skill content"));
        assert_eq!(results.rules.as_deref(), Some("rules content"));
        assert_eq!(results.style.as_deref(), Some("style content"));
        assert_eq!(results.do_dont.as_deref(), Some("dodont content"));
    }
}
