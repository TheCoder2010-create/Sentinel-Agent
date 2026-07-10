import { ModelProvider, type ChatMessage, type StreamCallbacks } from './provider-interface.js';

export class OpenAICompatibleProvider extends ModelProvider {
  private apiKey: string | undefined;
  private baseUrl: string;
  private displayName: string;

  constructor(baseUrl: string, apiKey: string | undefined, displayName: string) {
    super();
    this.baseUrl = baseUrl.replace(/\/+$/, '');
    this.apiKey = apiKey;
    this.displayName = displayName;
  }

  async stream(
    modelId: string,
    messages: ChatMessage[],
    callbacks: StreamCallbacks,
    signal?: AbortSignal,
  ): Promise<void> {
    if (!this.apiKey) {
      callbacks.onError(`${this.displayName} API key missing — set the corresponding env var`);
      return;
    }

    try {
      const response = await fetch(`${this.baseUrl}/chat/completions`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${this.apiKey}`,
        },
        body: JSON.stringify({
          model: modelId,
          messages: messages.map(m => ({ role: m.role, content: m.content })),
          stream: true,
        }),
        signal,
      });

      if (!response.ok) {
        const body = await response.text().catch(() => '');
        callbacks.onError(
          `${this.displayName} request failed: ${response.status}${body ? ` — ${body.slice(0, 300)}` : ''}`,
          `HTTP_${response.status}`,
        );
        return;
      }

      const reader = response.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          const trimmed = line.trim();
          if (!trimmed || !trimmed.startsWith('data:')) continue;
          const data = trimmed.slice(5).trim();
          if (data === '[DONE]') {
            callbacks.onDone();
            return;
          }
          try {
            const parsed = JSON.parse(data);
            const text = parsed.choices?.[0]?.delta?.content || '';
            if (text) callbacks.onChunk(text);
          } catch {
            // skip malformed JSON lines
          }
        }
      }
      callbacks.onDone();
    } catch (err: unknown) {
      if (typeof err === 'object' && err !== null && (err as DOMException).name === 'AbortError') {
        callbacks.onDone();
        return;
      }
      const message = err instanceof Error ? err.message : String(err);
      callbacks.onError(`${this.displayName} request failed: ${message}`);
    }
  }
}
