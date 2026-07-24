# GPU Access & Inference Optimization

## Low-end hardware (i3, 4-8 GB RAM, no GPU)

Everything works. The agent detects the hardware and degrades gracefully:

| Resource | Detection | Action |
|----------|-----------|--------|
| No GPU | `nvidia-smi` / `rocminfo` fails | CPU-only path: `llama.cpp` with no GPU offload |
| ≤8 GB RAM | `sysinfo` memory probe | Use Q2_K quantized models only. Pick a model ≤1B parameters |
| i3 CPU | CPU model / core count | Disable speculative decoding. Disable Flash Attention. Use simple greedy decoding |
| Too slow for task | Wall-clock heuristic | "This requires a GPU. Provision a cloud instance? [y/N]" |

**Minimum requirements for the agent itself:** 128 MB RAM, any CPU with a terminal. The TUI (ratatui) and agent loop are negligible.

**Minimum requirements for local inference:** 4 GB RAM, any x86-64 CPU with AVX2. TinyLlama 1.1B at Q2_K runs at ~5 tok/s on a 4-core i3 — slow but functional. For any serious workload, the agent offers cloud GPU provisioning as a one-command fallback.

## GPU access by platform

```
User intent → Agent → Resource Detector → GPU Scheduler
                                            │
                          ┌─────────────────┼─────────────────┐
                          ▼                 ▼                 ▼
                     Local GPU         Cloud GPU         CPU fallback
                  (CUDA/Metal/ROCm)  (Modal/RunPod)    (llama.cpp)
                          │                 │                 │
                          ▼                 ▼                 ▼
                    ┌──────────┐     ┌───────────┐     ┌──────────┐
                    │  Ollama   │     │ Container │     │  GGUF    │
                    │  (llama)  │     │ (CUDA env) │     │ (quant)  │
                    └──────────┘     └───────────┘     └──────────┘
```

| Platform | Detection | Backend | Toolchain |
|----------|-----------|---------|-----------|
| Linux (NVIDIA) | `nvidia-smi` | CUDA | cuBLAS, TensorRT-LLM, vLLM |
| Linux (AMD) | `rocminfo` | ROCm | ROCm backend, HIP |
| macOS (Apple Silicon) | `system_profiler SPDisplaysDataType` | Metal | MPS, Metal GPU |
| Windows (NVIDIA) | `nvidia-smi` / `wmic` | CUDA | cuBLAS, DirectML |
| Windows (other) | `dxdiag` | DirectML | DirectML EP |

The agent auto-detects the available backend and selects the optimal inference engine without user configuration.

---

## Improving inference speed

| Technique | Speed-up | VRAM | Quality impact | Auto-detectable |
|-----------|----------|------|---------------|:-:|
| **4-bit quantization** (GGUF q4_k_m, AWQ, GPTQ) | 2-4× | 4× less | <5% | Yes (VRAM < model size) |
| **Speculative decoding** (small draft + target model) | 2-3× | +draft model | None | Yes (if VRAM available) |
| **Flash Attention 2** | 1.5-2× | O(n) instead of O(n²) | None | Yes (GPU compute ≥ 8.0) |
| **PagedAttention / vLLM** | 1.5-3× | less fragmentation | None | Yes (CUDA only) |
| **Prompt caching** (KV-cache reuse) | 2-5× repeated | +1 system prompt | None | Always |
| **Tensor parallelism** (≥2 GPUs) | ~N× | split | None | Yes (detect multi-GPU) |
| **Continuous batching** (vLLM) | 2-4× | per request | None | Yes |

**Rule of thumb:** if VRAM < model FP16 size → apply 4-bit quantization. If VRAM has headroom → enable speculative decoding with a 2-3× smaller draft model. If GPU is CUDA compute ≥ 8.0 → enable Flash Attention.

---

## Improving output quality

| Technique | How it works | Quality gain | Cost |
|-----------|-------------|-------------|------|
| **Structured generation** | Constrain output to JSON/schema (LMQL, outlines, llama.cpp grammars) | Eliminates parsing errors | ~5% slower |
| **Self-consistency** | Run N times with temperature > 0, vote on answers | +5-15% accuracy on reasoning | N× compute |
| **Speculative rejection** | Verify each token with a small reward model before accepting | Fewer hallucinations | +10-20% per token |
| **RAG + GPU reranking** | Retrieve top-k, rerank with cross-encoder (BGE, Cohere) | Better context grounding | +200ms per query |
| **Prompt engineering** | Auto-optimize system prompt with few-shot examples | 5-20% task improvement | Zero runtime cost |
| **Ensemble decoding** | Run 3-5 models, pick best output by confidence | More robust answers | 3-5× compute |

---

## Auto-tuning flow

When user requests a task, the agent:

```
1. Detect GPU → vendor, VRAM, compute cap, driver version
2. Measure model size → FP16 size vs available VRAM
3. If VRAM < model → auto-enable 4-bit quantization
4. If VRAM has ≥2 GB headroom → enable speculative decoding
5. If CUDA compute ≥ 8.0 → enable Flash Attention
6. If user asks for "quality" → enable self-consistency (N=3)
7. If user asks for "speed" → skip quality extras
8. If structure needed → enable structured generation
```

All decisions happen transparently. The user only specifies the task and a preference (speed vs. quality).

---

## Implementation sketch

```rust
struct InferenceConfig {
    quantization: Option<QuantMethod>,
    speculative_decoding: bool,
    flash_attention: bool,
    paged_attention: bool,
    prompt_caching: bool,
    structured_output: Option<Schema>,
}

fn auto_tune(vram_gb: f64, model_size_gb: f64, compute_cap: u32) -> InferenceConfig {
    // rules engine — applied before every experiment
}
```

This is the brain of the `/run` command: parse the user's intent, tune the inference stack to match their hardware, execute, and return results with no manual flags.
