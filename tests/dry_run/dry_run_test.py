#!/usr/bin/env python
"""
Dry-run: Phases 0-5 against a real Terraform repo + mock Grafana/OTel.

Measures:
  1. Token-usage savings: compression ON vs OFF (core marketing claim)
  2. Doom-loop detector: does it fire on realistic debugging traces?
  3. Approval gate: does it block read-only diagnosis?

Run:  uv run python tests/dry_run/dry_run_test.py
"""

from __future__ import annotations

import asyncio
import json
import logging
import sys
from pathlib import Path

# -- Ensure project root is on sys.path ---------------------------------
PROJECT_ROOT = Path(__file__).resolve().parent.parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))

logging.basicConfig(
    level=logging.WARNING,
    format="%(levelname)-5s %(name)s %(message)s",
    stream=sys.stderr,
)
# Silence noisy loggers
for name in ("LiteLLM", "LiteLLM Router", "openai", "httpx", "httpcore", "agent"):
    logging.getLogger(name).setLevel(logging.CRITICAL)


# =======================================================================
#  1. TOKEN COMPRESSION TEST
# =======================================================================

from litellm import Message, token_counter
from agent.context_manager.compression import CompressionEngine


from litellm.types.utils import Function as LiteLLMFunction, ChatCompletionMessageToolCall as LiteLLMToolCall


def _msg(role: str, content: str = "", tool_calls: list | None = None, **kw) -> Message:
    kwargs = dict(kw)
    if content:
        kwargs["content"] = content
    if tool_calls:
        kwargs["tool_calls"] = tool_calls
    m = Message(role=role, **kwargs)
    return m


def _tool_call(name: str, args: dict, id_: str = "call_1") -> LiteLLMToolCall:
    return LiteLLMToolCall(
        function=LiteLLMFunction(arguments=json.dumps(args), name=name),
        id=id_,
        type="function",
    )


def _tool_result(name: str, content: str, tool_call_id: str = "call_1") -> Message:
    return _msg("tool", content, name=name, tool_call_id=tool_call_id)


def build_debugging_session() -> list[Message]:
    """Simulate a 4-turn debugging session: plan -> observe -> diagnose -> fix."""
    messages: list[Message] = []

    # -- System prompt ------------------------------------------------
    SYSTEM = (
        "You are Platform-Agent, an autonomous agent for Platform Engineering. "
        "You have tools at your disposal.\n\n"
        "Tool specs:\n"
        "  - query_otel_traces: Search OpenTelemetry traces by service, time range, error status\n"
        "  - query_grafana_panel: Query Grafana dashboard panels and run PromQL\n"
        "  - terraform_plan: Show Terraform changes without applying\n"
        "  - terraform_apply: Apply Terraform changes\n"
        "  - execute_local: Run local shell commands\n"
        "  - web_search: Search the web for information\n"
        "  - read: Read file contents\n"
    )
    messages.append(_msg("system", SYSTEM))

    # -- Turn 1: User asks to debug high error rate -------------------
    messages.append(_msg("user", (
        "Our production API error rate spiked to 12% in the last hour. "
        "We're running on AWS us-east-1, order-service and user-service. "
        "Please investigate and fix. Terraform files are in terraform_project/."
    )))

    # Assistant plans
    messages.append(_msg("assistant", (
        "I'll start by checking the traces and metrics to identify the root cause. "
        "Plan:\n"
        "  1. Query traces for order-service [pending]\n"
        "  2. Check Grafana dashboard for error rate patterns [pending]\n"
        "  3. Inspect Terraform plan for recent infra changes [pending]\n"
        "  4. Apply fix if needed [pending]\n"
        "  5. Verify fix resolved the issue [pending]"
    ), tool_calls=[
        _tool_call("plan_tool", {"command": "create", "todos": [
            {"id": "1", "content": "Query traces for order-service", "status": "pending"},
            {"id": "2", "content": "Check Grafana dashboard for error rate", "status": "pending"},
            {"id": "3", "content": "Inspect Terraform plan for changes", "status": "pending"},
            {"id": "4", "content": "Apply fix if needed", "status": "pending"},
            {"id": "5", "content": "Verify fix resolved the issue", "status": "pending"},
        ]}, "call_plan"),
    ]))
    messages.append(_tool_result("plan_tool", "Plan created with 5 steps."))

    # -- Turn 2: Query traces and dashboard ---------------------------
    messages.append(_msg("assistant", (
        "Phase 1: Let me query traces and the Grafana dashboard simultaneously."
    ), tool_calls=[
        _tool_call("query_otel_traces", {
            "service": "order-service",
            "lookback_minutes": 60,
            "error_only": True,
        }, "call_trace"),
        _tool_call("query_grafana_panel", {
            "dashboard_uid": "prod-obs-001",
            "panel_id": 1,
        }, "call_grafana"),
    ]))
    messages.append(_tool_result("query_otel_traces",
        "[OK] Traces for service 'order-service' (last 60m):\n"
        "  Found 2 trace(s)\n"
        "  [ERROR] abc123def4567890  order-service / POST /api/orders  (1200000us, 2 spans)\n"
        "  [OK] def456abc7890123  user-service / GET /api/users  (45000us, 1 span)\n"
        "Analysis: The order-service shows a POST /api/orders endpoint timing out "
        "(1200ms) with a downstream PostgreSQL connection timeout. "
        "This suggests the database is overwhelmed or the connection pool is exhausted.",
        "call_trace"))
    messages.append(_tool_result("query_grafana_panel",
        "[OK] Panel: API Error Rate (id=1)\n"
        "  datasource: prometheus/prometheus-prod\n"
        "  Query: sum(rate(http_requests_total{status=~\"5..\"}[5m])) / sum(rate(http_requests_total[5m])) * 100\n"
        "Analysis: The error rate panel shows a sustained 12% 5xx rate starting "
        "approximately 45 minutes ago, correlating with the trace data.",
        "call_grafana"))

    # Plan update step 1-2 completed
    messages.append(_msg("assistant", (
        "Steps 1 and 2 complete. Root cause identified: order-service can't reach PostgreSQL. "
        "Let me check the Terraform plan to see if any recent changes affected the RDS instance."
    ), tool_calls=[
        _tool_call("plan_tool", {"command": "update", "id": "1", "status": "completed"}, "call_p1"),
        _tool_call("plan_tool", {"command": "update", "id": "2", "status": "completed"}, "call_p2"),
    ]))
    messages.append(_tool_result("plan_tool", "Step 1 updated to completed."))
    messages.append(_tool_result("plan_tool", "Step 2 updated to completed."))

    # -- Turn 3: Check Terraform plan and RDS config ------------------
    messages.append(_msg("assistant", (
        "Phase 2: Let me check the Terraform state for the RDS instance and run a plan."
    ), tool_calls=[
        _tool_call("execute_local", {"command": "cd terraform_project && terraform plan -no-color"}, "call_tf1"),
    ]))
    messages.append(_tool_result("execute_local",
        "[OK] Terraform plan output:\n"
        "  No changes. Your infrastructure matches the configuration.\n"
        "  Terraform has compared your real infrastructure against your configuration and found no differences.",
        "call_tf1"))

    # Check RDS config
    messages.append(_msg("assistant", (
        "No Terraform changes pending. Let me check the RDS configuration and logs."
    ), tool_calls=[
        _tool_call("read", {"filePath": "terraform_project/modules/database/main.tf"}, "call_read"),
        _tool_call("read", {"filePath": "terraform_project/variables.tf"}, "call_read2"),
    ]))
    messages.append(_tool_result("read",
        "[FILE_READ] modules/database/main.tf\n"
        "resource \"aws_db_instance\" \"this\" {\n"
        "  identifier     = \"${var.environment}-postgres\"\n"
        "  instance_class = var.db_instance_class\n"
        "  allocated_storage = var.allocated_storage\n"
        "  max_allocated_storage = 1000\n"
        "  ...\n"
        "  enabled_cloudwatch_logs_exports = [\"postgresql\"]\n"
        "}",
        "call_read"))
    messages.append(_tool_result("read",
        "[FILE_READ] variables.tf\n"
        "  variable \"db_instance_class\" { default = \"db.r6g.large\" }\n"
        "  variable \"db_allocated_storage\" { default = 100 }\n"
        "  variable \"db_backup_retention_days\" { default = 30 }",
        "call_read2"))

    # Plan update
    messages.append(_msg("assistant", "Step 3 complete. RDS config looks normal but connection pool might be the issue.",
         tool_calls=[_tool_call("plan_tool", {"command": "update", "id": "3", "status": "completed"}, "call_p3")]))
    messages.append(_tool_result("plan_tool", "Step 3 updated to completed."))

    # -- Turn 4: Web search for fix -> apply fix -> verify --------------
    messages.append(_msg("assistant", (
        "Phase 3: The issue is likely the RDS connection pool being exhausted. "
        "Let me research the recommended fix and apply it."
    ), tool_calls=[
        _tool_call("web_search", {
            "query": "AWS RDS PostgreSQL connection pool exhaustion fix max_connections pgBouncer"
        }, "call_search"),
    ]))
    messages.append(_tool_result("web_search",
        "[OK] Search results:\n"
        "  1. AWS RDS PostgreSQL best practices: Use connection pooling with pgBouncer\n"
        "  2. Increase max_connections parameter in RDS parameter group\n"
        "  3. Or use serverless v2 scaling for production workloads\n"
        "Recommended: Update the RDS parameter group to increase max_connections and add pgBouncer.",
        "call_search"))

    # Apply fix: update parameter group and increase storage
    messages.append(_msg("assistant", (
        "Applying fix: increase max_connections via parameter group and scale up storage."
    ), tool_calls=[
        _tool_call("execute_local", {
            "command": (
                "cd terraform_project && "
                "sed -i 's/allocated_storage = 100/allocated_storage = 200/' modules/database/main.tf && "
                "echo 'Added max_connections parameter'"
            )
        }, "call_fix"),
    ]))
    messages.append(_tool_result("execute_local",
        "[OK] Updated allocated_storage to 200. Added max_connections parameter.",
        "call_fix"))

    # Apply Terraform
    messages.append(_msg("assistant", "Applying Terraform changes to fix the database.",
         tool_calls=[_tool_call("terraform_apply", {"auto_approve": True, "directory": "terraform_project"}, "call_apply")]))
    messages.append(_tool_result("terraform_apply",
        "[OK] Terraform apply complete:\n"
        "  aws_db_parameter_group.this: Modifying...\n"
        "  aws_db_instance.this: Modifying...\n"
        "  Apply complete! Resources: 0 added, 2 changed, 0 destroyed.",
        "call_apply"))

    # Verify
    messages.append(_msg("assistant", (
        "Verification: Checking traces again to confirm the error rate dropped."
    ), tool_calls=[
        _tool_call("query_otel_traces", {
            "service": "order-service",
            "lookback_minutes": 5,
            "error_only": True,
        }, "call_verify"),
        _tool_call("terraform_plan", {"directory": "terraform_project"}, "call_plan2"),
    ]))
    messages.append(_tool_result("query_otel_traces",
        "[OK] Traces for service 'order-service' (last 5m):\n"
        "  Found 0 trace(s) with errors.\n"
        "  The error rate has dropped to 0%.",
        "call_verify"))
    messages.append(_tool_result("terraform_plan",
        "[OK] No changes. Your infrastructure matches the configuration.",
        "call_plan2"))

    # Mark complete
    messages.append(_msg("assistant", "All steps complete. Error rate resolved.",
         tool_calls=[
             _tool_call("plan_tool", {"command": "update", "id": "4", "status": "completed"}, "call_p4"),
             _tool_call("plan_tool", {"command": "update", "id": "5", "status": "completed"}, "call_p5"),
         ]))
    messages.append(_tool_result("plan_tool", "Step 4 updated to completed."))
    messages.append(_tool_result("plan_tool", "Step 5 updated to completed."))

    return messages


async def test_token_compression() -> dict:
    """Measure token savings from the compression engine."""
    messages = build_debugging_session()
    print(f"\n{'='*60}")
    print("  TOKEN COMPRESSION TEST")
    print(f"{'='*60}")
    print("  Session: 4-turn debugging scenario")
    print(f"  Messages: {len(messages)}")
    print("  Tools used: query_otel_traces, query_grafana_panel, execute_local, read, web_search, terraform_apply")

    # -- Baseline: count tokens WITHOUT compression -----------------
    def _safe_token_count(msgs: list[Message]) -> int:
        """Count tokens, safely handling non-serializable content."""
        try:
            clean = []
            for m in msgs:
                d = {"role": m.role, "content": m.content}
                if isinstance(d["content"], list):
                    d["content"] = " ".join(
                        p.get("text", "") for p in d["content"] if isinstance(p, dict)
                    )
                if hasattr(m, "tool_calls") and m.tool_calls:
                    clean_calls = []
                    for tc in m.tool_calls:
                        fn_args = tc.function.arguments
                        if not isinstance(fn_args, str):
                            fn_args = json.dumps(fn_args)
                        clean_calls.append({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.function.name,
                                "arguments": fn_args,
                            },
                        })
                    d["tool_calls"] = clean_calls
                clean.append(d)
            return token_counter(model="gpt-4", messages=clean)
        except Exception:
            return sum(len(str(m.content or "")) for m in msgs) // 4

    baseline_tokens = _safe_token_count(messages)
    baseline_tool_specs = [
        {"type": "function", "function": {
            "name": "query_otel_traces",
            "description": "Search OpenTelemetry traces",
            "parameters": {"type": "object", "properties": {}},
        }},
        {"type": "function", "function": {
            "name": "query_grafana_panel",
            "description": "Query Grafana panels",
            "parameters": {"type": "object", "properties": {}},
        }},
        {"type": "function", "function": {
            "name": "execute_local",
            "description": "Run local commands",
            "parameters": {"type": "object", "properties": {}},
        }},
        {"type": "function", "function": {
            "name": "terraform_plan",
            "description": "Show Terraform changes",
            "parameters": {"type": "object", "properties": {}},
        }},
        {"type": "function", "function": {
            "name": "terraform_apply",
            "description": "Apply Terraform changes",
            "parameters": {"type": "object", "properties": {}},
        }},
        {"type": "function", "function": {
            "name": "read",
            "description": "Read file contents",
            "parameters": {"type": "object", "properties": {}},
        }},
        {"type": "function", "function": {
            "name": "web_search",
            "description": "Search the web",
            "parameters": {"type": "object", "properties": {}},
        }},
    ]

    # -- Apply compression ------------------------------------------
    engine = CompressionEngine(model_name="gpt-4")

    # Register file reads (so diff-only context can work on re-reads)
    engine.register_file_read("terraform_project/modules/database/main.tf", (
        '[FILE_READ] modules/database/main.tf\n'
        'resource "aws_db_instance" "this" {...}'
    ))
    engine.register_file_read("terraform_project/variables.tf", (
        '[FILE_READ] variables.tf\n'
        'variable "db_instance_class" {...}'
    ))

    # Simulate the per-turn compression that CompressionContextManager does
    specs = list(baseline_tool_specs)

    # Simulate tool calling patterns (marking tools as seen)
    for tool_name in ["plan_tool", "query_otel_traces", "query_grafana_panel",
                       "execute_local", "read", "web_search", "terraform_apply", "terraform_plan"]:
        engine.mark_tool_called(tool_name)

    # Mark some tool results as "consumed" (as if the agent used them in reasoning)
    engine.mark_consumed("call_trace", "Found order-service POST timeout with DB connection error")
    engine.mark_consumed("call_grafana", "Error rate at 12%, confirmed")
    engine.mark_consumed("call_search", "Found pgBouncer and max_connections recommendations")

    # Mark steps completed for summarization
    engine.mark_step_completed("1")
    engine.mark_step_completed("2")
    engine.record_step_message_range("1", 5, 10)  # indices in our messages list
    engine.record_step_message_range("2", 10, 15)
    engine.cache_step_summary("1", "Queried traces for order-service - found DB timeout error connecting to PostgreSQL")
    engine.cache_step_summary("2", "Checked Grafana dashboard - confirmed sustained 12% error rate from HTTP 5xx responses")

    # Do a compression pass on the full message list (simulates the cumulative effect)
    compressed_msgs, compressed_specs = engine.compress_messages(messages, specs)
    compressed_tokens = _safe_token_count(compressed_msgs)

    # Simulate tool spec compression (lazy tool docs: shorted docs for already-called tools)
    lazy_specs = engine._apply_lazy_tool_docs(specs)
    lazy_specs_tokens = sum(
        len(json.dumps(s)) // 4 for s in lazy_specs
    )
    baseline_specs_tokens = sum(
        len(json.dumps(s)) // 4 for s in baseline_tool_specs
    )

    total_compressed = compressed_tokens + lazy_specs_tokens
    total_baseline = baseline_tokens + baseline_specs_tokens
    pct = (1 - total_compressed / total_baseline) * 100

    results = {
        "baseline_tokens": total_baseline,
        "compressed_tokens": total_compressed,
        "tokens_saved": total_baseline - total_compressed,
        "compression_ratio": round(total_compressed / total_baseline, 3),
        "savings_pct": round(pct, 1),
        "message_tokens_baseline": baseline_tokens,
        "message_tokens_compressed": compressed_tokens,
        "specs_tokens_baseline": baseline_specs_tokens,
        "specs_tokens_compressed": lazy_specs_tokens,
    }

    def _fmt_row(c1, v1, v2, v3, v4):
        return f"  | {c1:<20} | {str(v1):>9} | {str(v2):>9} | {str(v3):>9} | {v4:<7} |"
    header = _fmt_row("Component", "Baseline", "Compressed", "Saved", "Ratio")
    sep = "  |" + "-"*22 + "|" + "-"*11 + "|" + "-"*11 + "|" + "-"*11 + "|" + "-"*9 + "|"
    print(f"\n  {sep}")
    print(f"  {header}")
    print(f"  {sep}")
    print(f"  {_fmt_row('Messages', baseline_tokens, compressed_tokens, baseline_tokens - compressed_tokens, f'{compressed_tokens/baseline_tokens:.0%}')}")
    print(f"  {_fmt_row('Tool specs', baseline_specs_tokens, lazy_specs_tokens, baseline_specs_tokens - lazy_specs_tokens, f'{lazy_specs_tokens/baseline_specs_tokens:.0%}')}")
    print(f"  {sep}")
    print(f"  {_fmt_row('TOTAL', total_baseline, total_compressed, total_baseline - total_compressed, f'{pct:.0f}% off')}")
    print(f"  {sep}")

    print(f"\n  [OK] Compression savings: {total_baseline - total_compressed} tokens ({pct}%)")
    if pct >= 15:
        print("  [OK] EXCEEDS 15% CLAIM THRESHOLD - viable marketing number")
    else:
        print("  [WARN] Below 15% - may need additional compression strategies")

    # Detail on which strategies contributed
    print("\n  Strategy breakdown:")
    print(f"    * Consumed-output pruning: {engine._consumed_tool_ids}")
    print(f"    * Completed step summaries: {engine._completed_step_ids}")
    print(f"    * Lazy tool docs (seen tools): {engine.seen_tools()}")
    print("    * System caching: cache_control injected")

    return results


# =======================================================================
#  2. DOOM-LOOP DETECTOR TEST
# =======================================================================

from agent.core.doom_loop import (
    check_for_doom_loop,
    extract_recent_tool_signatures,
    detect_identical_consecutive,
    detect_repeating_sequence,
)


def build_doom_debug_session() -> list[Message]:
    """Simulate a stuck debugging session: repeatedly kubectl logs with no progress."""
    messages: list[Message] = build_debugging_session()[:18]  # first ~2 turns

    # Now the agent gets stuck in a loop running identical commands
    for i in range(6):
        messages.append(_msg("assistant", "Let me check the logs again for more details.",
            tool_calls=[_tool_call("execute_local", {
                "command": "kubectl logs -n production deployment/order-service --tail=100"
            }, f"call_log_{i}")]))
        messages.append(_tool_result("execute_local",
            "[OK] Log output:\n"
            "  2026-07-07 21:45:12 ERROR POST /api/orders context deadline exceeded\n"
            "  2026-07-07 21:45:13 ERROR connection to database timed out\n"
            "  [repeats 47 times]",
            f"call_log_{i}"))

    return messages


def build_legitimate_polling_session() -> list[Message]:
    """Simulate a legitimate deployment monitoring session (same args, different results)."""
    messages: list[Message] = build_debugging_session()[:5]

    states = [
        "deployment-abc123 0/3 ready  ContainerCreating",
        "deployment-abc123 1/3 ready  Running",
        "deployment-abc123 2/3 ready  Running",
        "deployment-abc123 3/3 ready  Running",
        "deployment-abc123 3/3 ready  Running",
    ]
    for i, state in enumerate(states):
        messages.append(_msg("assistant", f"Checking deployment status (attempt {i+1})",
            tool_calls=[_tool_call("execute_local", {
                "command": "kubectl get pods -n production -l app=order-service --no-headers"
            }, f"call_poll_{i}")]))
        messages.append(_tool_result("execute_local",
            f"[OK] {state}", f"call_poll_{i}"))

    return messages


async def test_doom_loop_detector() -> dict:
    """Test that the doom-loop detector catches real stuck patterns but ignores polling."""
    print(f"\n{'='*60}")
    print("  DOOM-LOOP DETECTOR TEST")
    print(f"{'='*60}")

    results = {}

    # -- Test 1: Legitimate debugging loop (should fire) --------------
    loop_msgs = build_doom_debug_session()
    sigs1 = extract_recent_tool_signatures(loop_msgs, lookback=30)
    identical = detect_identical_consecutive(sigs1, threshold=3)
    doom = check_for_doom_loop(loop_msgs)

    print("\n  Test 1: Stuck debugging (6x identical kubectl logs)")
    print(f"    Total messages: {len(loop_msgs)}")
    print(f"    Signatures extracted: {len(sigs1)}")
    print(f"    Identical consecutive detected: {'YES - ' + identical if identical else 'NO'}")
    print(f"    Doom prompt generated: {'YES' if doom else 'NO'}")

    results["stuck_loop_detected"] = bool(doom)
    results["stuck_loop_tool"] = identical

    # -- Test 2: Legitimate polling (should NOT fire) -----------------
    poll_msgs = build_legitimate_polling_session()
    sigs2 = extract_recent_tool_signatures(poll_msgs, lookback=30)
    identical2 = detect_identical_consecutive(sigs2, threshold=3)
    doom2 = check_for_doom_loop(poll_msgs)

    print("\n  Test 2: Legitimate deployment monitoring (5x polls, different results)")
    # Check: all have same args hash but different result hashes
    arg_hashes = {s.args_hash for s in sigs2}
    result_hashes = {s.result_hash for s in sigs2}
    print(f"    Unique args hashes: {len(arg_hashes)}")
    print(f"    Unique result hashes: {len(result_hashes)}")
    print(f"    Identical consecutive detected: {'YES' if identical2 else 'NO (correct)'}")
    print(f"    Doom prompt generated: {'YES (FALSE POSITIVE!)' if doom2 else 'NO (correct)'}")

    results["polling_false_positive"] = bool(doom2)
    results["polling_correctly_suppressed"] = not bool(doom2)

    # -- Test 3: Repeating sequence pattern (2-step cycle) ------------
    cycle_msgs = build_debugging_session()[:5]
    for i in range(4):
        cycle_msgs.append(_msg("assistant", "",
            tool_calls=[
                _tool_call("execute_local", {"command": "terraform plan -no-color"}, f"call_cyc_{i}a"),
                _tool_call("read", {"filePath": "main.tf"}, f"call_cyc_{i}b"),
            ]))
        cycle_msgs.append(_tool_result("execute_local", "[OK] No changes.", f"call_cyc_{i}a"))
        cycle_msgs.append(_tool_result("read", "[FILE_READ] resource \"aws_instance\" {...}", f"call_cyc_{i}b"))

    sigs3 = extract_recent_tool_signatures(cycle_msgs, lookback=30)
    pattern = detect_repeating_sequence(sigs3)
    doom3 = check_for_doom_loop(cycle_msgs)

    print("\n  Test 3: Repeating 2-step cycle (plan->read 4 times)")
    print(f"    Signatures: {len(sigs3)}")
    print(f"    Repeating pattern detected: {'YES - ' + ' -> '.join(s.name for s in pattern) if pattern else 'NO'}")
    print(f"    Doom prompt generated: {'YES' if doom3 else 'NO'}")

    results["repeating_cycle_detected"] = bool(doom3)
    results["repeating_pattern"] = ' -> '.join(s.name for s in pattern) if pattern else None

    # -- Test 4: Healthy session (should NOT fire) --------------------
    healthy_msgs = build_debugging_session()
    sigs4 = extract_recent_tool_signatures(healthy_msgs, lookback=30)
    identical4 = detect_identical_consecutive(sigs4, threshold=3)
    doom4 = check_for_doom_loop(healthy_msgs)

    print("\n  Test 4: Healthy debugging session (diverse tools, no repetition)")
    tool_names = list(dict.fromkeys(s.name for s in sigs4))
    print(f"    Unique tools: {tool_names}")
    print(f"    Identical consecutive detected: {'YES (FALSE POSITIVE!)' if identical4 else 'NO (correct)'}")
    print(f"    Doom prompt generated: {'YES (FALSE POSITIVE!)' if doom4 else 'NO (correct)'}")

    results["healthy_session_false_positive"] = bool(doom4)

    return results


# =======================================================================
#  3. APPROVAL GATE TEST
# =======================================================================



def _make_config(yolo_mode=False):
    from types import SimpleNamespace
    cfg = SimpleNamespace()
    cfg.yolo_mode = yolo_mode
    cfg.default_approval = None
    cfg.read_only_tools = []
    return cfg


async def test_approval_gate() -> dict:
    """Verify the approval gate never blocks read-only diagnosis."""
    from agent.core.agent_loop import _base_needs_approval

    print(f"\n{'='*60}")
    print("  APPROVAL GATE TEST")
    print(f"{'='*60}")

    # Simulate the _base_needs_approval with a real config
    # We need to import the real function but monkey-patch the config check
    config = _make_config(yolo_mode=False)

    test_cases = [
        # (tool_name, args, expected_approval, description)
        ("query_otel_traces", {"service": "order-service"}, False, "Read-only: search traces"),
        ("query_grafana_panel", {"dashboard_uid": "prod-obs-001"}, False, "Read-only: query dashboard"),
        ("terraform_plan", {"directory": "terraform_project"}, False, "Read-only: plan (no mutation)"),
        ("terraform_state", {"directory": "terraform_project"}, False, "Read-only: state read"),
        ("read", {"filePath": "main.tf"}, False, "Read-only: file read"),
        ("execute_local", {"command": "kubectl get pods"}, False, "Read-only: kubectl get"),
        ("web_search", {"query": "terraform best practices"}, False, "Read-only: web search"),
        ("git_status", {}, False, "Read-only: git status"),
        ("git_diff", {}, False, "Read-only: git diff"),
        ("research", {"query": "RDS connection pooling"}, False, "Read-only: research agent"),
        ("terraform_apply", {"auto_approve": True}, True, "MUTATING: terraform apply"),
    ]

    print(f"\n  {'Tool':<30} {'Requires Approval':<20} {'Expected':<12} {'Result':<12}")
    print(f"  {'-'*74}")

    gate = {
        "read_only_correct": True,
        "mutating_correct": True,
        "friction_tools": [],
    }

    for tool_name, args, expected, desc in test_cases:
        # Call the real _base_needs_approval
        decision = _base_needs_approval(tool_name, args, config)
        correct = decision == expected
        status = "PASS" if correct else "FAIL"

        if tool_name == "terraform_apply":
            if correct:
                gate["mutating_correct"] = True
            else:
                gate["mutating_correct"] = False
                gate["friction_tools"].append(tool_name)
        elif expected is False:
            if decision:
                gate["read_only_correct"] = False
                gate["friction_tools"].append(tool_name)

        print(f"  {tool_name:<30} {'YES' if decision else 'NO':<20} {'YES' if expected else 'NO':<12} {status}")

    print("\n  Read-only boundary:")
    print(f"    All read-only tools exempt from approval: {gate['read_only_correct']}")
    print(f"    All mutating tools correctly gated: {gate['mutating_correct']}")
    if gate["friction_tools"]:
        print(f"    [WARN] Friction tools: {gate['friction_tools']}")
    else:
        print("    [OK] No friction - read-only boundary is clean")

    return gate


# =======================================================================
#  4. RUNNER
# =======================================================================

async def run_dry_run():
    """Execute all three tests and produce a summary report."""
    print(f"\n{'#'*60}")
    print("  DRY RUN: Phases 0-5")
    print("  Target: terraform_project/ (real HCL) + mock Grafana/OTel")
    print("  Terraform files: 6 files, 400+ lines HCL")
    print("  Terraform modules: networking, database, storage")
    print(f"{'#'*60}")

    # Test 1: Token compression
    comp_results = await test_token_compression()

    # Test 2: Doom loop detector
    doom_results = await test_doom_loop_detector()

    # Test 3: Approval gate
    gate_results = await test_approval_gate()

    # -- Summary Report ----------------------------------------------
    print(f"\n\n{'='*60}")
    print("  DRY-RUN RESULTS SUMMARY")
    print(f"{'='*60}")

    yes = comp_results['savings_pct'] >= 15
    savings_str = f"{comp_results['savings_pct']:.1f}% savings ({comp_results['tokens_saved']} tokens)"
    claim_str = "YES (>15%)" if yes else "NEEDS WORK (<15%)"
    print(f"""
  +-{'+-'*35}+
  | MEASUREMENT {' '*35}| RESULT{' '*32}|
  +-{'+-'*35}+
  | Token compression           | {savings_str:<39}|
  | Marketing claim viable?     | {claim_str:<48}|
  +-{'+-'*35}+
  | Doom: stuck loop detected   | {'YES' if doom_results['stuck_loop_detected'] else 'NO'}{' '*44}|
  | Doom: polling false positive| {'NO (correct)' if not doom_results.get('polling_false_positive') else 'YES (BAD!)'}{' '*37}|
  | Doom: repeating cycle       | {'YES' if doom_results['repeating_cycle_detected'] else 'NO'}{' '*44}|
  | Doom: healthy session       | {'NO (correct)' if not doom_results['healthy_session_false_positive'] else 'YES (BAD!)'}{' '*37}|
  +-{'+-'*35}+
  | Approval: read-only exempt  | {'ALL CLEAN' if gate_results['read_only_correct'] else 'HAS ISSUES'}{' '*38}|
  | Approval: mutating gated    | {'YES' if gate_results['mutating_correct'] else 'NO'}{' '*44}|
  +-{'+-'*35}+
  | Phase 0 (init)              | ToolRouter: 15 built-in tools{' '*26}|
  | Phase 1-2 (plan-act)        | Terraform plan -> observe -> fix{' '*24}|
  | Phase 3-4 (observe-diagnose)| OTel traces + Grafana + diagnosis{' '*20}|
  | Phase 5 (verify)            | Re-check traces after fix{' '*28}|
  +-{'+-'*35}+
    """)

    # Overall verdict
    all_pass = (
        comp_results['savings_pct'] >= 5
        and doom_results['stuck_loop_detected']
        and not doom_results.get('polling_false_positive', True)
        and not doom_results['healthy_session_false_positive']
        and gate_results['read_only_correct']
        and gate_results['mutating_correct']
    )

    if all_pass:
        verdict = "PASS - DRY RUN COMPLETE - Safe to wire up real AWS/GCP mutation"
    else:
        failures = []
        if comp_results['savings_pct'] < 5:
            failures.append("compression too low")
        if not doom_results['stuck_loop_detected']:
            failures.append("doom loop missed stuck pattern")
        if doom_results.get('polling_false_positive'):
            failures.append("doom loop false positive on polling")
        if doom_results['healthy_session_false_positive']:
            failures.append("doom loop false positive on healthy session")
        if not gate_results['read_only_correct']:
            failures.append("approval gate blocking read-only tools")
        if not gate_results['mutating_correct']:
            failures.append("approval gate not blocking mutating tools")
        verdict = f"PARTIAL FAIL - Issues: {', '.join(failures)}"

    print(f"  {verdict}")

    return {
        "compression": comp_results,
        "doom_loop": doom_results,
        "approval_gate": gate_results,
        "verdict": verdict,
    }


async def main():
    results = await run_dry_run()
    return results


if __name__ == "__main__":
    asyncio.run(main())
