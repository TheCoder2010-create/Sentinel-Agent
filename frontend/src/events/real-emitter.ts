import { EventEmitter } from 'node:events';
import { getProviderForModel, modelIdToApiModel, getMissingKeyMessage } from '../providers/index.js';
import type { ChatMessage } from '../providers/provider-interface.js';

export type AgentEventType =
  | 'ready' | 'processing'
  | 'assistant_chunk' | 'assistant_message' | 'assistant_stream_end'
  | 'tool_call' | 'tool_output' | 'tool_log' | 'tool_state_change'
  | 'approval_required' | 'turn_complete' | 'interrupted' | 'error'
  | 'compacted' | 'plan_generated' | 'step_completed' | 'observation';

export interface AgentEvent {
  type: AgentEventType;
  data?: Record<string, unknown>;
  timestamp: number;
}

export interface PlanItem {
  id: string;
  content: string;
  status: 'pending' | 'in_progress' | 'completed';
}

export class RealEventEmitter extends EventEmitter {
  private _running = false;
  private abortController: AbortController | null = null;
  private modelId = '';
  private history: ChatMessage[] = [];

  start(modelId?: string, _apiKey?: string, _providerId?: string) {
    if (this._running) return;
    this._running = true;
    this.modelId = modelId || '';
    this.history = [];

    if (this.modelId) {
      const missingKey = getMissingKeyMessage(this.modelId);
      if (missingKey) {
        this.emit('event', {
          type: 'error',
          data: { message: `Cannot start: ${missingKey} — set it in your .env file` },
          timestamp: Date.now(),
        } as AgentEvent);
      }
    }

    this.emit('event', {
      type: 'ready',
      timestamp: Date.now(),
    } as AgentEvent);
  }

  stop() {
    if (!this._running) return;
    this._running = false;
    if (this.abortController) {
      this.abortController.abort();
      this.abortController = null;
    }
  }

  isRunning() {
    return this._running;
  }

  send(text: string) {
    if (!this._running) return;
    if (!this.modelId) {
      this.emit('event', {
        type: 'error',
        data: { message: 'No model selected' },
        timestamp: Date.now(),
      } as AgentEvent);
      return;
    }

    this.emit('event', {
      type: 'processing',
      data: { message: 'Thinking...' },
      timestamp: Date.now(),
    } as AgentEvent);

    this.history.push({ role: 'user', content: text });

    this.abortController = new AbortController();
    const signal = this.abortController.signal;

    let provider: ReturnType<typeof getProviderForModel>;
    try {
      provider = getProviderForModel(this.modelId);
    } catch (err: unknown) {
      const message = err instanceof Error ? err.message : String(err);
      this.emit('event', {
        type: 'error',
        data: { message },
        timestamp: Date.now(),
      } as AgentEvent);
      return;
    }

    const apiModel = modelIdToApiModel(this.modelId);
    const messages = [...this.history];

    provider.stream(apiModel, messages, {
      onChunk: (chunkText: string) => {
        this.emit('event', {
          type: 'assistant_chunk',
          data: { text: chunkText },
          timestamp: Date.now(),
        } as AgentEvent);
      },
      onDone: () => {
        this.abortController = null;
        this.history.push({ role: 'assistant', content: '' });
        this.emit('event', {
          type: 'assistant_stream_end',
          timestamp: Date.now(),
        } as AgentEvent);
        this.emit('event', {
          type: 'turn_complete',
          data: { summary: 'Response complete', turnCount: 1 },
          timestamp: Date.now(),
        } as AgentEvent);
      },
      onError: (message: string, code?: string) => {
        this.abortController = null;
        this.emit('event', {
          type: 'error',
          data: { message, code },
          timestamp: Date.now(),
        } as AgentEvent);
      },
    }, signal);
  }

  sendCommand(cmd: string) {
    if (cmd === '/new') {
      this.history = [];
      this.emit('event', {
        type: 'compacted',
        data: { tokensBefore: 0, tokensAfter: 0 },
        timestamp: Date.now(),
      } as AgentEvent);
    } else {
      this.emit('event', {
        type: 'tool_log',
        data: { tool: 'system', message: `Command received: ${cmd}` },
        timestamp: Date.now(),
      } as AgentEvent);
    }
  }

  sendApproval(approvals: Array<{ id: string; approved: boolean }>) {
    this.emit('event', {
      type: 'tool_log',
      data: { tool: 'system', message: `Approval: ${approvals.map(a => `${a.id}=${a.approved}`).join(', ')}` },
      timestamp: Date.now(),
    } as AgentEvent);
  }
}
