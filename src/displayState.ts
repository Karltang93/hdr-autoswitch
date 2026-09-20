import type { HdrStatePayload, MonitorInfo, TargetMonitor } from './types.ts';

export function monitorReady(monitor: MonitorInfo): monitor is MonitorInfo & { device_path: string } {
  return !!monitor.device_path?.trim()
    && monitor.identity_status === 'ready' && !monitor.identity_error
    && monitor.hdr_state_known && !monitor.state_error && monitor.is_hdr_supported;
}

export function manualControlAvailable(status: HdrStatePayload, loaded: boolean): boolean {
  return loaded && status.manual_control.status === 'available' && !status.inventory_stale;
}

export function manualScopeAvailable(scope: TargetMonitor, monitors: MonitorInfo[]): boolean {
  if (scope.kind === 'all') return monitors.some(monitorReady);
  return scope.kind === 'monitor' && monitors.some((monitor) =>
    monitorReady(monitor) && monitor.device_path.toLowerCase() === scope.device_path.toLowerCase());
}

export function monitorMode(monitor: MonitorInfo): HdrStatePayload['scope_hdr_state'] {
  if (!monitor.hdr_state_known || monitor.state_error) return 'unknown';
  return monitor.is_hdr_enabled ? 'hdr' : 'sdr';
}

export const scopeVisuals = {
  hdr: {
    panel: 'bg-[#180e10] border-[#f55a6b] neon-glow-coral',
    dial: 'bg-[#221314] border-[#f55a6b] shadow-[0_0_25px_rgba(245,90,107,0.5)] scale-105',
    badge: 'bg-[#f55a6b] text-[#0f0b0b] border-[#f55a6b]',
    dot: 'bg-[#0f0b0b] animate-status-pulse',
  },
  sdr: {
    panel: 'bg-[#120d0e] border-[#5accf5]/40',
    dial: 'bg-[#170f10] border-[#5accf5]/40',
    badge: 'bg-[#221314] text-[#5accf5] border-[#5accf5]/40',
    dot: 'bg-[#5accf5]',
  },
  mixed: {
    panel: 'bg-amber-950/20 border-amber-400/70',
    dial: 'bg-amber-950/30 border-amber-400',
    badge: 'bg-amber-400/15 text-amber-200 border-amber-400/70',
    dot: 'bg-amber-300',
  },
  unknown: {
    panel: 'bg-slate-900/30 border-slate-500 border-dashed',
    dial: 'bg-slate-900/40 border-slate-500 border-dashed',
    badge: 'bg-slate-800/50 text-slate-300 border-slate-500 border-dashed',
    dot: 'border border-slate-300',
  },
};
