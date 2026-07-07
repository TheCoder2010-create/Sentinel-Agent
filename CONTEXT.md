# Platform-Agent

> **GitHub**: `Single-Core-Labs/Sentinel-Agent`
> **CLI**: `platform-agent` (entry point: `agent.main:cli`)

---

## Architecture

```ascii
┌─────────────────────────────────────────────────────────────┐
│                       User / CLI                            │
└──────┬──────────────────────────────────────────┬───────────┘
       │ Operations (OpType)                       │ Events
       ↓ (user_input, exec_approval, undo,         ↑
  submission_queue  compact, new, resume, shutdown) event_queue
       │                                            │
       ↓                                            │
┌──────────────────────────────────────────────────────┐
│              submission_loop (agent_loop.py)          │
│  ┌────────────────────────────────────────────────┐  │
│  │  process_submission() — route OpType to        │  │
│  │  handler                                        │  │
│  └────────────────────────────────────────────────┘  │
│                        ↓                             │
│  ┌────────────────────────────────────────────────┐  │
│  │           Handlers.run_agent()                 │  │
│  │                                                │  │
│  │  ┌──────────────────────────────────────────┐  │  │
│  │  │  Session                                 │  │  │
│  │  │  ┌──────────────────────────────────┐    │  │  │
│  │  │  │  ContextManager                  │    │  │  │
│  │  │  │  • Message history               │    │  │  │
│  │  │  │    (litellm.Message[])           │    │  │  │
│  │  │  │  • Auto-compaction at 90%        │    │  │  │
│  │  │  │    of model_max_tokens           │    │  │  │
│  │  │  └──────────────────────────────────┘    │  │  │
│  │  │                                          │  │  │
│  │  │  ┌──────────────────────────────────┐    │  │  │
│  │  │  │  ToolRouter                      │    │  │  │
│  │  │  │  • GitHub code search / read     │    │  │  │
│  │  │  │  • Local filesystem tools        │    │  │  │
│  │  │  │  • Planning / Notify             │    │  │  │
│  │  │  │  • MCP server tools (dynamic)    │    │  │  │
│  │  │  └──────────────────────────────────┘    │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  │                                                │  │
│  │  ┌──────────────────────────────────────────┐  │  │
│  │  │  Doom Loop Detector                      │  │  │
│  │  │  • 3+ identical consecutive tool calls   │  │  │
│  │  │  • Repeating sequences                   │  │  │
│  │  │  • Injects corrective prompt             │  │  │
│  │  └──────────────────────────────────────────┘  │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

## Agentic Loop

```ascii
User Message → [ContextManager]
  ╔═════════════════════════════════════════════╗
  ║  Iteration Loop (max 300)                   ║
  ║  1. Cancel check → compact check            ║
  ║  2. Doom-loop detection                     ║
  ║  3. litellm.acompletion()                   ║
  ║     ↓                                       ║
  ║  4. Has tool_calls? ─No──> emit done        ║
  ║     │ Yes                                   ║
  ║  5. Validate args + add to context          ║
  ║  6. Approval check per tool                 ║
  ║  7. Execute (parallel if no approval needed)║
  ║  8. Add results → loop                      ║
  ╚═════════════════════════════════════════════╝
```

## Operations (OpType)

| OpType | Handler | Description |
|---|---|---|
| `USER_INPUT` | `Handlers.run_agent()` | Main agentic loop |
| `EXEC_APPROVAL` | `Handlers.exec_approval()` | User responds to approval request |
| `UNDO` | `Handlers.undo()` | Remove last complete turn |
| `COMPACT` | `_compact_and_notify()` | Force context compaction |
| `NEW` | `Handlers.new_conversation()` | Fresh chat |
| `RESUME` | `Handlers.resume()` | Reload saved session |
| `SHUTDOWN` | `Handlers.shutdown()` | Save + stop |

> `interrupt` is **not** an OpType — `session.cancel()` sets a flag, loop exits cleanly.
> Model switching (`/model`) is handled **outside** the loop in `main.py`.

## Events

`ready`, `processing`, `assistant_chunk`, `assistant_message`, `assistant_stream_end`,
`tool_call`, `tool_output`, `tool_log`, `tool_state_change`, `approval_required`,
`turn_complete`, `interrupted`, `error`, `compacted`, `undo_complete`, `new_complete`,
`resume_complete`, `shutdown`

## Tools

| Tool | Purpose |
|---|---|
| `research` | Sub-agent with read-only tools |
| `web_search` | DuckDuckGo search |
| `plan_tool` | Multi-step planning |
| `notify` | Slack notifications |
| `github_find_examples` / `github_list_repos` / `github_read_file` | GitHub code search |

Plus local tools (bash/read/write/edit) and dynamic MCP tools.

## Key Files

| Path | Purpose |
|---|---|
| `agent/main.py` | CLI entry, event listener, command dispatch |
| `agent/core/agent_loop.py` | submission_loop, handlers, agentic loop |
| `agent/core/session.py` | Session, OpType, Event |
| `agent/core/tools.py` | ToolRouter, ToolSpec, tool registration |
| `agent/core/doom_loop.py` | Repeat detection |
| `agent/core/model_switcher.py` | Model listing, probing, switching |
| `agent/context_manager/manager.py` | Message history, compaction |
| `agent/config.py` | Config dataclass |
| `agent/utils/terminal_display.py` | CLI rendering, theme |
| `agent/utils/particle_logo.py` | Startup particle animation |
| `agent/utils/crt_boot.py` | CRT-style boot sequence |
| `agent/utils/boot_timing.py` | Color interpolation helpers |

---

## Session Changes Log

### 1. Changed UI theme to blue
- **`agent/utils/boot_timing.py`**: `warm_gold_from_white()` → `blue_from_white()` (white→blue)
- **`agent/utils/particle_logo.py`**: All hold/final colors from `(255,200,80)` → `(80,160,255)`
- **`agent/utils/terminal_display.py`**: Theme colors, boot lines, init display, tool calls → blue
- **`agent/utils/crt_boot.py`**: Cursor, noise, scanlines → blue
- **`agent/main.py`**: Model picker heading → blue

### 2. Changed animations
- **`agent/utils/particle_logo.py`**: FPS 24→30, converge 0.9s→0.7s, more particles
- **`agent/utils/crt_boot.py`**: New glitch character set

### 3. Added model provider picker at startup
- **`agent/main.py`**: Added `_model_picker()` function called after `ready_event.wait()`
- Shows numbered list of 6 suggested models
- User enters number, custom model ID, or Enter to skip
- Calls `probe_and_switch_model()` on selection

### 4. Startup flow (current)
```
1. Particle logo: "WELCOME TO / PLATFORM-AGENT" (blue, ~2.5s)
2. CRT boot: "Welcome to Platform-Agent" + system info
3. Agent initialization
4. Model picker:
   1. anthropic/claude-opus-4.8:fal-ai  (Claude Opus 4.8)
   2. openai/gpt-5.5:fal-ai            (GPT-5.5)
   3. MiniMaxAI/MiniMax-M3:novita      (MiniMax M3)
   4. moonshotai/Kimi-K2.7-Code:novita (Kimi K2.7 Code)
   5. zai-org/GLM-5.2:novita           (GLM 5.2)
   6. deepseek-ai/DeepSeek-V4-Pro:novita (DeepSeek V4 Pro)
   0. Skip — keep default
   Enter number or paste model ID (Enter to skip):
5. Agent ready with selected model
```

### 5. Pushed to GitHub
- Remote: `https://github.com/Single-Core-Labs/Sentinel-Agent.git`
- Initial commit: all 199 files

---

## File inventory (agent/tools/)
- `docs_tools.py` — Documentation search tools
- `dataset_tools.py` — Dataset inspection
- `papers_tool.py` — Paper discovery
- `edit_utils.py` — String replacement helpers
- `github_find_examples.py` — GitHub example discovery
- `github_list_repos.py` — GitHub repo listing
- `github_read_file.py` — GitHub file reader
- `local_tools.py` — Local filesystem tools (bash/read/write/edit)
- `notify_tool.py` — Slack notifications
- `plan_tool.py` — Multi-step planning
- `research_tool.py` — Sub-agent delegation
- `types.py` — ToolResult type
- `utilities.py` — Job formatting helpers
- `web_search_tool.py` — DuckDuckGo search
