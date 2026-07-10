export interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

export interface StreamCallbacks {
  onChunk: (text: string) => void;
  onDone: () => void;
  onError: (message: string, code?: string) => void;
}

export abstract class ModelProvider {
  abstract stream(
    modelId: string,
    messages: ChatMessage[],
    callbacks: StreamCallbacks,
    signal?: AbortSignal,
  ): Promise<void>;
}
