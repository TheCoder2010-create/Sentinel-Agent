import { Box, Typography } from '@mui/material';
import { keyframes } from '@mui/system';
import { useAgentStore, type ActivityStatus } from '@/store/agentStore';

const shimmer = keyframes`
  0% { background-position: -100% center; }
  50% { background-position: 200% center; }
  100% { background-position: -100% center; }
`;

const TOOL_LABELS: Record<string, string> = {
  sandbox_create: 'Creating sandbox for code development, this might take 1-2 minutes',
  bash: 'Running command in sandbox',
  plan_tool: 'Planning',
  research: 'Researching',
};

/** Format raw research log into a clean status label. */
function formatResearchStatus(raw: string): string {
  const s = raw.replace(/^▸\s*/, '');
  const jsonStart = s.indexOf('{');
  const toolName = jsonStart > 0 ? s.slice(0, jsonStart).trim() : s.trim();
  const args: Record<string, string> = {};
  if (jsonStart > 0) {
    const jsonStr = s.slice(jsonStart);
    try {
      const parsed = JSON.parse(jsonStr);
      for (const [k, v] of Object.entries(parsed)) {
        if (typeof v === 'string') args[k] = v;
      }
    } catch {
      // JSON is likely truncated — extract complete "key": "value" pairs
      for (const m of jsonStr.matchAll(/"(\w+)":\s*"([^"]*)"/g)) {
        args[m[1]] = m[2];
      }
      // Also try to extract a truncated value for known keys if not found yet
      if (!args.query && !args.arxiv_id) {
        const partial = jsonStr.match(/"(query|arxiv_id)":\s*"([^"]*)/);
        if (partial) args[partial[1]] = partial[2];
      }
    }
  }

  if (toolName === 'github_find_examples') {
    const d = (args.keyword) || (args.repo);
    return d ? `Finding examples: ${d}` : 'Finding examples';
  }
  if (toolName === 'github_read_file') {
    const f = ((args.path) || '').split('/').pop();
    return f ? `Reading ${f}` : 'Reading file';
  }
  return 'Researching';
}

function statusLabel(status: ActivityStatus): string {
  switch (status.type) {
    case 'thinking': return 'Thinking';
    case 'streaming': return 'Writing';
    case 'tool': {
      if (status.toolName === 'research' && status.description) {
        return formatResearchStatus(status.description);
      }
      const base = status.description || TOOL_LABELS[status.toolName] || `Running ${status.toolName}`;
      if (status.toolName === 'bash' && status.description && /install/i.test(status.description)) {
        return `${base} — this can take a few minutes, sit tight`;
      }
      return base;
    }
    case 'waiting-approval': return 'Waiting for approval';
    case 'cancelled': return 'What should the agent do instead?';
    default: return '';
  }
}

export default function ActivityStatusBar() {
  const activityStatus = useAgentStore(s => s.activityStatus);

  if (activityStatus.type === 'idle') return null;

  const label = statusLabel(activityStatus);

  return (
    <Box sx={{ px: 2, py: 0.5, minHeight: 28, display: 'flex', alignItems: 'center' }}>
      <Typography
        sx={{
          fontFamily: 'monospace',
          fontSize: '0.72rem',
          fontWeight: 500,
          letterSpacing: '0.02em',
          background: 'linear-gradient(90deg, var(--muted-text) 30%, var(--text) 50%, var(--muted-text) 70%)',
          backgroundSize: '250% 100%',
          backgroundClip: 'text',
          WebkitBackgroundClip: 'text',
          WebkitTextFillColor: 'transparent',
          animation: `${shimmer} 4s ease-in-out infinite`,
        }}
      >
        {label}{activityStatus.type !== 'cancelled' && '…'}
      </Typography>
    </Box>
  );
}
