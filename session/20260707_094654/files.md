# Files

## Inspected
- `agent/main.py` — entire file; CLI entry, event_listener, command dispatch, token flow
- `agent/core/agent_loop.py` — submission_loop, handlers, agentic loop, approval policy
- `agent/core/session.py` — Session class, OpType, Event
- `agent/core/tools.py` — ToolRouter, ToolSpec, create_builtin_tools
- `agent/core/doom_loop.py` — doom loop detection
- `agent/core/model_switcher.py` — model listing, probing, switching
- `agent/core/hf_tokens.py` — token resolution
- `agent/core/hf_access.py` — whoami, org access
- `agent/context_manager/manager.py` — ContextManager, compaction
- `agent/config.py` — Config dataclass
- `agent/utils/terminal_display.py` — CLI rendering, theme, banner
- `agent/utils/particle_logo.py` — startup animation
- `agent/utils/crt_boot.py` — CRT boot sequence
- `agent/utils/boot_timing.py` — color interpolation
- `agent/tools/jobs_tool.py` — HF Jobs tool (uses HfApi)
- `agent/tools/hf_repo_files_tool.py` — inspected before deletion
- `agent/tools/hf_repo_git_tool.py` — inspected before deletion
- `agent/tools/research_tool.py` — sub-agent delegation
- `agent/tools/\__init__.py` — tool exports
- `backend/dependencies.py` — auth dependencies
- `backend/routes/agent.py` — dataset upload endpoint
- `backend/routes/auth.py` — OAuth flow
- `pyproject.toml` — project config
- `agent/prompts/system_prompt_v3.yaml` — active system prompt
- `agent/prompts/system_prompt_v2.yaml` — previous prompt

## Changed
- `agent/main.py` — removed token prompt, removed hf_repo display blocks, added _model_picker(), removed is_local_model_id import
- `agent/core/tools.py` — removed hf_repo_files/hf_repo_git imports + ToolSpec registrations
- `agent/core/agent_loop.py` — removed hf_repo_files/hf_repo_git approval rules
- `agent/tools/research_tool.py` — removed hf_repo_files from allowed tools + docs
- `agent/utils/boot_timing.py` — warm_gold_from_white → blue_from_white
- `agent/utils/particle_logo.py` — text changed, colors to blue, animation tweaks
- `agent/utils/terminal_display.py` — theme colors, boot lines, tool call colors → blue
- `agent/utils/crt_boot.py` — cursor/noise/scanline colors → blue, new glitch chars
- `backend/routes/agent.py` — removed 401 token check, added HF_TOKEN env fallback
- `CONTEXT.md` — written with full architecture

## Deleted
- `agent/tools/hf_repo_files_tool.py` — HF repo file management
- `agent/tools/hf_repo_git_tool.py` — HF repo git operations
- Various `__pycache__/` directories — stale bytecode cache

## Generated
- `CONTEXT.md` — full architecture documentation
- `session/20260707_094654/session_state.md` — session state
- `session/20260707_094654/timeline.md` — action log
- `session/20260707_094654/files.md` — file inventory
- `session/20260707_094654/handoff.md` — resume instructions
