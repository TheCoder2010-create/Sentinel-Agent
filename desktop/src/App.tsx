import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

function App() {
  const [prompt, setPrompt] = useState('');
  const [response, setResponse] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async () => {
    setLoading(true);
    try {
      const result = await invoke('run_agent', { prompt });
      setResponse(result as string);
    } catch (e) {
      setResponse(`Error: ${e}`);
    }
    setLoading(false);
  };

  return (
    <div style={{ padding: '2rem', fontFamily: 'system-ui, sans-serif' }}>
      <h1>Sentinel AI Desktop</h1>
      <textarea
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        placeholder="Enter your prompt..."
        rows={4}
        style={{ width: '100%', marginBottom: '1rem' }}
      />
      <button onClick={handleSubmit} disabled={loading}>
        {loading ? 'Processing...' : 'Send'}
      </button>
      {response && (
        <pre style={{ marginTop: '1rem', background: '#f5f5f5', padding: '1rem' }}>
          {response}
        </pre>
      )}
    </div>
  );
}

export default App;
