# Pipeline Agent (read → triage → draft → QA → send)

The pipeline agent wraps the standard `Agent` with a structured five-stage
workflow. Each stage is a self-contained agent run with its own system
instruction; between stages the message history accumulates and a checkpoint
is saved.

## Stages

```
  user input
      │
      ▼
┌──────────┐    checkpoint    ┌──────────┐    checkpoint
│  READ    │ ──────────────► │  TRIAGE  │ ──────────────► ...
│ gather   │                 │ analyze  │
│ context  │                 │ plan     │
└──────────┘                 └──────────┘
                                      ...
                              ┌──────────┐    checkpoint    ┌──────────┐
                              │   QA     │ ──────────────► │  SEND    │
                              │ verify   │                 │ finalize │
                              │ test     │                 │ present  │
                              └──────────┘                 └──────────┘
                                                                    │
                                                                    ▼
                                                             final result
```

| Stage | Purpose | Agent Behavior |
|-------|---------|----------------|
| **Read** | Gather context, explore codebase | read/glob/grep/search tools only. No changes. |
| **Triage** | Analyze findings, plan approach | Planning only, no implementation. |
| **Draft** | Implement the solution | Write/edit files freely. |
| **QA** | Review and verify | Read/test only. Fix issues. |
| **Send** | Finalize and present | Summarize changes, deliver result. |

## Checkpoints

After each stage completes, a `ThreadCheckpoint` is saved (messages, phase,
turn, iterations). If a stage fails with `rollback_on_error: true`, the thread
is restored to the last checkpoint.

## How It Works

`PipelineAgent::run_pipeline()` runs a modified agent loop:

1. **System prompt injection**: Before each stage, a system message with
   `## Pipeline Stage: {NAME}` and the stage instruction is appended.

2. **Same inner loop**: Each stage runs the same LLM-call → tool-execute
   loop as the base `Agent`, including malformed-call recovery, truncation
   detection, doom-loop detection, and context compaction.

3. **Stage transition**: When the agent produces a non-tool-call response,
   the stage is complete. If more stages remain, the pipeline advances;
   otherwise the final result is returned.

4. **Message accumulation**: All messages from all stages accumulate in the
   thread context. The agent in Triage sees everything from Read, etc.

## Code Location

- **`crates/sentinel-core/src/pipeline.rs`**: `PipelineStage`, `PipelineConfig`,
  `ThreadCheckpoint` (on `AgentThread`), `PipelineAgent`

## CLI Wiring (`sentinel-cli/src/exec.rs`)

```rust
let pipeline_agent = sentinel_core::pipeline::PipelineAgent::new(agent);
let result = pipeline_agent.run_pipeline(&mut thread, &prompt, approval.as_ref()).await;
```

The CLI prints the pipeline stages in the banner:
```
 Pipeline: read → triage → draft → QA → send
```

## Config

```rust
pub struct PipelineConfig {
    pub stages: Vec<PipelineStage>,       // default: [Read, Triage, Draft, QA, Send]
    pub save_checkpoints: bool,            // default: true
    pub rollback_on_error: bool,           // default: true
}
```

## API

```rust
// Create with default config
let pipeline = PipelineAgent::new(agent);

// Custom config
let config = PipelineConfig { stages: vec![PipelineStage::Read, PipelineStage::Draft], .. };
let pipeline = PipelineAgent::with_config(agent, config);

// Run
pipeline.run_pipeline(&mut thread, user_input, &approval_gate).await;

// Access inner agent
let agent: &Agent = pipeline.inner();
let agent: Agent = pipeline.into_inner();
```

## Thread Checkpoint API

```rust
thread.snapshot() -> ThreadCheckpoint    // save
thread.restore(&checkpoint)              // restore
```
