import { type ReactNode, useEffect, useMemo, useState } from 'react';
import {
  Box,
  Button,
  CircularProgress,
  Link,
  Popover,
  Tooltip,
  Typography,
} from '@mui/material';
import PaidOutlinedIcon from '@mui/icons-material/PaidOutlined';
import OpenInNewIcon from '@mui/icons-material/OpenInNew';
import { useSessionStore } from '@/store/sessionStore';
import {
  type UsageBucket,
  useUsageStore,
} from '@/store/usageStore';

function formatUsd(value: number | undefined): string {
  const amount = value ?? 0;
  if (amount > 0 && amount < 0.01) return '<$0.01';
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency: 'USD',
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(amount);
}

function formatCount(value: number | undefined): string {
  return new Intl.NumberFormat('en-US').format(value ?? 0);
}

function contextTokenCount(telemetry: UsageBucket | null | undefined): number | undefined {
  if (!telemetry) return undefined;
  if (telemetry.total_tokens > 0) {
    return Math.max(0, telemetry.total_tokens - telemetry.completion_tokens);
  }
  return (
    telemetry.prompt_tokens +
    telemetry.cache_read_tokens +
    telemetry.cache_creation_tokens
  );
}

function UsageRow({
  label,
  value,
  strong = false,
}: {
  label: string;
  value: string;
  strong?: boolean;
}) {
  return (
    <>
      <Typography variant="body2" color="text.secondary">
        {label}
      </Typography>
      <Typography
        variant="body2"
        sx={{ fontWeight: strong ? 700 : 400, fontVariantNumeric: 'tabular-nums' }}
      >
        {value}
      </Typography>
    </>
  );
}

function UsageGrid({ children }: { children: ReactNode }) {
  return (
    <Box
      sx={{
        display: 'grid',
        gridTemplateColumns: '1fr auto',
        columnGap: 2,
        rowGap: 0.5,
        mt: 0.75,
      }}
    >
      {children}
    </Box>
  );
}

function AccountUsageSection({
  title,
  telemetry,
}: {
  title: string;
  telemetry: UsageBucket | null | undefined;
}) {
  return (
    <Box sx={{ py: 1 }}>
      <Typography variant="caption" sx={{ color: 'text.secondary', fontWeight: 700 }}>
        {title}
      </Typography>
      <UsageGrid>
        <UsageRow
          label="LLM calls"
          value={formatCount(telemetry?.llm_calls)}
        />
        <UsageRow
          label="Input tokens"
          value={formatCount(contextTokenCount(telemetry))}
        />
        <UsageRow
          label="Output tokens"
          value={formatCount(telemetry?.completion_tokens)}
        />
      </UsageGrid>
    </Box>
  );
}

export default function UsageMeter() {
  const activeSessionId = useSessionStore((state) => state.activeSessionId);
  const activeSessionYoloSpend = useSessionStore((state) => {
    const active = state.sessions.find((session) => session.id === state.activeSessionId);
    return active?.autoApprovalEstimatedSpendUsd ?? null;
  });
  const { usage, isLoading, error, fetchUsage } = useUsageStore();
  const [anchorEl, setAnchorEl] = useState<HTMLElement | null>(null);

  useEffect(() => {
    void fetchUsage(activeSessionId);
  }, [activeSessionId, activeSessionYoloSpend, fetchUsage]);

  const sessionTotal = usage?.session?.total_usd;
  const links = useMemo(() => usage?.links ?? {}, [usage?.links]);
  const open = Boolean(anchorEl);

  return (
    <>
      <Tooltip title="Usage">
        <Button
          size="small"
          variant="outlined"
          startIcon={isLoading ? <CircularProgress size={14} /> : <PaidOutlinedIcon fontSize="small" />}
          onClick={(event) => setAnchorEl(event.currentTarget)}
          sx={{
            minWidth: { xs: 58, sm: 84 },
            height: 32,
            px: { xs: 0.75, sm: 1 },
            borderColor: 'divider',
            color: 'text.secondary',
            fontVariantNumeric: 'tabular-nums',
            '& .MuiButton-startIcon': { mr: { xs: 0.25, sm: 0.5 } },
            '&:hover': { borderColor: 'primary.main', color: 'primary.main' },
          }}
        >
          {sessionTotal == null ? 'Usage' : formatUsd(sessionTotal)}
        </Button>
      </Tooltip>
      <Popover
        open={open}
        anchorEl={anchorEl}
        onClose={() => setAnchorEl(null)}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        transformOrigin={{ vertical: 'top', horizontal: 'right' }}
        slotProps={{
          paper: {
            sx: {
              width: 320,
              maxWidth: 'calc(100vw - 24px)',
              maxHeight: 'calc(100vh - 24px)',
              overflowY: 'auto',
              p: 2,
              border: '1px solid',
              borderColor: 'divider',
            },
          },
        }}
      >
        <Typography variant="subtitle2" sx={{ fontWeight: 800 }}>
          Usage
        </Typography>
        <Typography variant="caption" color="text.secondary">
          Estimated from HF account usage per session.
        </Typography>

        {error ? (
          <Typography variant="body2" color="error" sx={{ mt: 1.5 }}>
            {error}
          </Typography>
        ) : (
          <>
            <AccountUsageSection
              title="Current session"
              telemetry={usage?.session ?? null}
            />
          </>
        )}

        <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, pt: 1 }}>
          {links.jobs_pricing && (
            <Link href={links.jobs_pricing} target="_blank" rel="noopener noreferrer" underline="hover" sx={{ display: 'inline-flex', alignItems: 'center', gap: 0.25, fontSize: '0.75rem' }}>
              Jobs pricing <OpenInNewIcon sx={{ fontSize: 12 }} />
            </Link>
          )}
        </Box>
      </Popover>
    </>
  );
}
