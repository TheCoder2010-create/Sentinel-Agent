/**
 * Smoke test — run with: npx tsx test-provider.ts
 * Set the API key for the provider you want to test first, e.g.:
 *   $env:OPENAI_API_KEY="sk-..." ; npx tsx test-provider.ts openai/gpt-4o
 */
import { getProviderForModel, modelIdToApiModel } from './src/providers/index.js';

const modelId = process.argv[2] || 'openai/gpt-4o';
const prompt  = process.argv[3] || 'Hello — respond with exactly one short sentence.';

const provider = getProviderForModel(modelId);
const apiModel = modelIdToApiModel(modelId);

console.error(`Testing model: ${modelId}  →  API model: ${apiModel}`);

let full = '';
const start = Date.now();

await provider.stream(apiModel, [{ role: 'user', content: prompt }], {
  onChunk(text) {
    full += text;
    process.stdout.write(text);
  },
  onDone() {
    const elapsed = ((Date.now() - start) / 1000).toFixed(1);
    console.error(`\n\n✅ Done in ${elapsed}s  (${full.length} chars)`);
    if (!full) console.error('⚠  Empty response');
    process.exit(0);
  },
  onError(msg, code) {
    console.error(`\n❌ ${code ? `[${code}] ` : ''}${msg}`);
    process.exit(1);
  },
});
