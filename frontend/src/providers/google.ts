import { ModelProvider, type ChatMessage, type StreamCallbacks, type ToolDef, type CompletionResult, type ToolCallData } from './provider-interface.js';

const DEBUG = typeof process !== 'undefined' && (process.env['DEBUG'] === '1' || process.argv.includes('--debug'));

function env(name: string): string | undefined {
  return typeof process !== 'undefined' ? process.env[name] : undefined;
}

function debugLog(...args: unknown[]) {
  if (DEBUG) console.debug('[Google]', ...args);
}

export class GoogleProvider extends ModelProvider {
  private apiKey: string | undefined;

  constructor() {
    super();
    this.apiKey = env('GOOGLE_AI_STUDIO_API_KEY');
  }

  async complete(
    modelId: string,
    messages: ChatMessage[],
    tools?: ToolDef[],
    signal?: AbortSignal,
  ): Promise<CompletionResult> {
    if (!this.apiKey) {
      return { content: '', toolCalls: [], finishReason: 'error' };
    }

    const geminiModel = modelId.replace(/^(google\/|gemini\/)/, '');
    const url = `https://generativelanguage.googleapis.com/v1beta/models/${geminiModel}:generateContent?key=${this.apiKey}`;

    const body: Record<string, unknown> = {
      contents: messages.map(m => ({
        role: m.role === 'assistant' ? 'model' : m.role === 'tool' ? 'user' : m.role,
        parts: m.role === 'tool'
          ? [{ text: `[Tool result for ${m.name ?? m.tool_call_id ?? 'unknown'}]: ${m.content}` }]
          : [{ text: m.content }],
      })),
    };

    if (tools && tools.length > 0) {
      body.tools = [{
        functionDeclarations: tools.map(t => ({
          name: t.name,
          description: t.description,
          parameters: t.inputSchema,
        })),
      }];
    }

    try {
      debugLog('complete() POST gemini model=%s', modelId);
      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
        signal,
      });

      if (!response.ok) {
        const errBody = await response.text().catch(() => '');
        throw new Error(`Gemini request failed: ${response.status}${errBody ? ` — ${errBody.slice(0, 300)}` : ''}`);
      }

      const data = (await response.json()) as {
        candidates?: Array<{
          content?: { parts?: Array<{ text?: string; functionCall?: { name: string; args: Record<string, unknown> } }> };
          finishReason?: string;
        }>;
      };

      const candidate = data.candidates?.[0];
      if (!candidate) return { content: '', toolCalls: [], finishReason: 'stop' };

      const parts = candidate.content?.parts ?? [];
      let content = '';
      const toolCalls: ToolCallData[] = [];

      for (const part of parts) {
        if (part.text) {
          content += part.text;
        }
        if (part.functionCall) {
          toolCalls.push({
            id: `fc_${toolCalls.length}`,
            name: part.functionCall.name,
            arguments: part.functionCall.args ?? {},
          });
        }
      }

      debugLog('complete() finishReason=%s contentLen=%d toolCalls=%d', candidate.finishReason, content.length, toolCalls.length);

      return { content, toolCalls, finishReason: candidate.finishReason ?? 'stop' };
    } catch (err: unknown) {
      if (typeof err === 'object' && err !== null && (err as DOMException).name === 'AbortError') {
        debugLog('complete() aborted');
        return { content: '', toolCalls: [], finishReason: 'interrupted' };
      }
      debugLog('complete() error: %s', err instanceof Error ? err.message : String(err));
      throw err;
    }
  }

  async stream(
    modelId: string,
    messages: ChatMessage[],
    callbacks: StreamCallbacks,
    signal?: AbortSignal,
  ): Promise<void> {
    if (!this.apiKey) {
      callbacks.onError('Google AI Studio API key missing — set GOOGLE_AI_STUDIO_API_KEY');
      return;
    }

    const geminiModel = modelId.replace(/^(google\/|gemini\/)/, '');
    const url = `https://generativelanguage.googleapis.com/v1beta/models/${geminiModel}:streamGenerateContent?alt=sse&key=${this.apiKey}`;

    try {
      debugLog('stream() POST gemini model=%s', modelId);
      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          contents: messages.map(m => ({
            role: m.role === 'assistant' ? 'model' : m.role,
            parts: [{ text: m.content }],
          })),
        }),
        signal,
      });

      if (!response.ok) {
        const body = await response.text().catch(() => '');
        callbacks.onError(
          `Gemini request failed: ${response.status}${body ? ` — ${body.slice(0, 300)}` : ''}`,
          `HTTP_${response.status}`,
        );
        return;
      }

      const reader = response.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = '';
      let chunksReceived = 0;

      debugLog('stream() first byte received');

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed.startsWith('data:')) continue;
          const data = trimmed.slice(5).trim();
          if (!data || data === '[DONE]') continue;
          try {
            const parsed = JSON.parse(data);
            const text = parsed.candidates?.[0]?.content?.parts?.[0]?.text || '';
            if (text) {
              chunksReceived++;
              callbacks.onChunk(text);
            }
            if (parsed.candidates?.[0]?.finishReason && parsed.candidates[0].finishReason !== 'STOP_UNSPECIFIED') {
              debugLog('stream() finishReason=%s chunks=%d', parsed.candidates[0].finishReason, chunksReceived);
              if (chunksReceived === 0) {
                callbacks.onError('Gemini returned zero content chunks', 'EMPTY_STREAM');
                return;
              }
              callbacks.onDone();
              return;
            }
          } catch (err: unknown) {
            debugLog('stream() JSON parse error: %s line="%s"', err instanceof Error ? err.message : String(err), data.slice(0, 100));
          }
        }
      }

      debugLog('stream() connection closed chunks=%d', chunksReceived);
      if (chunksReceived === 0) {
        callbacks.onError('Gemini returned zero content chunks (no finishReason)', 'EMPTY_STREAM');
        return;
      }
      callbacks.onDone();
    } catch (err: unknown) {
      if (typeof err === 'object' && err !== null && (err as DOMException).name === 'AbortError') {
        debugLog('stream() aborted');
        callbacks.onDone();
        return;
      }
      const message = err instanceof Error ? err.message : String(err);
      debugLog('stream() error: %s', message);
      callbacks.onError(`Gemini request failed: ${message}`);
    }
  }
}
