import { ModelProvider, type ChatMessage, type StreamCallbacks } from './provider-interface.js';

function env(name: string): string | undefined {
  return typeof process !== 'undefined' ? process.env[name] : undefined;
}

export class GoogleProvider extends ModelProvider {
  private apiKey: string | undefined;

  constructor() {
    super();
    this.apiKey = env('GOOGLE_AI_STUDIO_API_KEY');
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
            if (text) callbacks.onChunk(text);
            if (parsed.candidates?.[0]?.finishReason && parsed.candidates[0].finishReason !== 'STOP_UNSPECIFIED') {
              callbacks.onDone();
              return;
            }
          } catch {
            // skip
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
      callbacks.onError(`Gemini request failed: ${message}`);
    }
  }
}
