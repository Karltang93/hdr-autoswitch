import type { HdrStatePayload, ManualControlError, ManualRequestOrigin, MonitorInfo, MonitorInventorySnapshot, TargetMonitor } from './types.ts';

function revision(value: string): bigint | null {
  return /^(0|[1-9]\d*)$/.test(value) ? BigInt(value) : null;
}

export class DisplayObservationOrder {
  private statusRevision = -1n;
  private statusInventoryRevision = -1n;
  private inventoryRevision = 0n;
  private loadedInventoryRevision = -1n;

  acceptStatus(status: Pick<HdrStatePayload, 'status_revision' | 'inventory_revision'>): boolean {
    const nextStatus = revision(status.status_revision);
    const nextInventory = revision(status.inventory_revision);
    if (nextStatus === null || nextInventory === null
      || nextStatus < this.statusRevision || nextInventory < this.inventoryRevision) return false;
    this.statusRevision = nextStatus;
    this.statusInventoryRevision = nextInventory;
    this.inventoryRevision = nextInventory;
    return true;
  }

  acceptInventory(snapshot: Pick<MonitorInventorySnapshot, 'inventory_revision'>): boolean {
    const next = revision(snapshot.inventory_revision);
    if (next === null || next < this.inventoryRevision) return false;
    this.inventoryRevision = next;
    this.loadedInventoryRevision = next;
    return true;
  }

  invalidateInventory(): void {
    this.loadedInventoryRevision = -1n;
  }

  get needsInventory(): boolean {
    return this.loadedInventoryRevision < this.inventoryRevision;
  }

  get statusCurrent(): boolean {
    return this.statusInventoryRevision === this.inventoryRevision;
  }
}

function scopeKey(scope: TargetMonitor): string {
  switch (scope.kind) {
    case 'all': return 'all';
    case 'monitor': return `monitor:${scope.device_path.toLowerCase()}`;
    case 'needs_confirmation': return `legacy:${scope.legacy_runtime_id}`;
  }
}

// Native result errors live in ordered actor status. Only local/admission/transport
// errors need this channel; any newer same-scope actor result supersedes them.
export class ManualFeedbackOrder {
  private manualRevision = 0n;
  private scopeRevisions = new Map<string, bigint>();
  private pendingErrors = new Map<string, ManualControlError>();

  capture(scope: TargetMonitor): ManualRequestOrigin {
    return { scope, after_revision: this.manualRevision.toString() };
  }

  acceptStatus(status: Pick<HdrStatePayload, 'manual_revision' | 'manual_results'>): boolean {
    const next = revision(status.manual_revision);
    if (next === null || next < this.manualRevision) return false;
    this.manualRevision = next;
    let changed = false;
    for (const result of status.manual_results) {
      const observed = revision(result.revision);
      if (observed === null || observed > next) continue;
      const key = scopeKey(result.scope);
      if (observed < (this.scopeRevisions.get(key) ?? -1n)) continue;
      this.scopeRevisions.set(key, observed);
      const error = this.pendingErrors.get(key);
      if (error && observed > BigInt(error.after_revision)) {
        this.pendingErrors.delete(key);
        changed = true;
      }
    }
    return changed;
  }

  acceptError(error: ManualControlError): boolean {
    const after = revision(error.after_revision);
    const key = scopeKey(error.scope);
    if (after === null || after < (this.scopeRevisions.get(key) ?? -1n)) return false;
    const previous = this.pendingErrors.get(key);
    if (previous && (BigInt(previous.after_revision) > after
      || (previous.after_revision === error.after_revision && previous.message === error.message))) return false;
    this.pendingErrors.set(key, error);
    return true;
  }

  errors(): string[] {
    return [...new Set([...this.pendingErrors.values()].map((error) => error.message))];
  }
}

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
