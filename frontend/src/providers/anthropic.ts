import { ModelProvider, type ChatMessage, type StreamCallbacks } from './provider-interface.js';

function env(name: string): string | undefined {
  return typeof process !== 'undefined' ? process.env[name] : undefined;
}

export class AnthropicProvider extends ModelProvider {
  private apiKey: string | undefined;

  constructor() {
    super();
    this.apiKey = env('ANTHROPIC_API_KEY');
  }

  async stream(
    modelId: string,
    messages: ChatMessage[],
    callbacks: StreamCallbacks,
    signal?: AbortSignal,
  ): Promise<void> {
    if (!this.apiKey) {
      callbacks.onError('Anthropic API key missing — set ANTHROPIC_API_KEY');
      return;
    }

    try {
      const response = await fetch('https://api.anthropic.com/v1/messages', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': this.apiKey,
          'anthropic-version': '2023-06-01',
        },
        body: JSON.stringify({
          model: modelId,
          max_tokens: 8192,
          messages: messages.map(m => ({ role: m.role, content: m.content })),
          stream: true,
        }),
        signal,
      });

      if (!response.ok) {
        const body = await response.text().catch(() => '');
        callbacks.onError(
          `Anthropic request failed: ${response.status}${body ? ` — ${body.slice(0, 300)}` : ''}`,
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
          if (!trimmed || !trimmed.startsWith('data: ')) continue;
          const raw = trimmed.slice(6);
          try {
            const parsed = JSON.parse(raw);
            if (parsed.type === 'content_block_delta' && parsed.delta?.type === 'text_delta') {
              const text = parsed.delta.text || '';
              if (text) callbacks.onChunk(text);
            } else if (parsed.type === 'message_stop') {
              callbacks.onDone();
              return;
            } else if (parsed.type === 'error') {
              callbacks.onError(parsed.error?.message || 'Anthropic API error', parsed.error?.type);
              return;
            } else if (parsed.type === 'message_start' && parsed.message?.content) {
              for (const block of parsed.message.content) {
                if (block.type === 'text' && block.text) {
                  callbacks.onChunk(block.text);
                }
              }
            } else if (parsed.type === 'content_block_start' && parsed.content_block?.type === 'text') {
              if (parsed.content_block.text) {
                callbacks.onChunk(parsed.content_block.text);
              }
            }
          } catch {
            // skip malformed JSON
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
      callbacks.onError(`Anthropic request failed: ${message}`);
    }
  }
}
