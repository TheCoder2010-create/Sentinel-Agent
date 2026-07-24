# Local Model Setup (`/local`)

One-command Ollama installation, model pull, and auto-configuration directly from the TUI — no terminal browsing required.

## Usage

```
/local              # Auto-detect hardware and pull a suitable model
/local llama3.2:3b  # Pull a specific model
/local phi          # Or any model from the Ollama library
```

## What it does

| Step | Action |
|------|--------|
| 1 | Detects OS (Windows/macOS/Linux), CPU cores, RAM, GPU |
| 2 | Checks if Ollama is installed — downloads + installs if missing |
| 3 | Starts `ollama serve` in background, waits for ready |
| 4 | Checks if the model is already pulled; pulls if needed |
| 5 | Reports completion with the model name for `/model` |

## Auto-selected models

| Hardware | Default model | RAM estimate |
|----------|---------------|-------------|
| GPU + ≥8 GB | `llama3.2:3b` | ~2 GB VRAM |
| ≥4 GB RAM, no GPU | `llama3.2:1b` | ~1 GB |
| Low-end | `tinyllama` | ~500 MB |

Override with `/local <model-name>`.

## System detection

- **OS**: `cfg!(target_os)` + `std::env::consts::ARCH`
- **CPU cores**: `std::thread::available_parallelism()`
- **RAM**: `wmic` (Windows), `sysctl hw.memsize` (macOS), `/proc/meminfo` (Linux)
- **GPU**: `nvidia-smi` / `rocminfo` / `system_profiler SPDisplaysDataType`
- **Ollama installed**: `which ollama` / `where ollama`

## Implementation

`crates/sentinel-ai-tui/src/local_model.rs` — all blocking work runs inside `tokio::task::spawn_blocking` so the TUI stays responsive. Installs are silent (Windows uses `/verysilent`).
