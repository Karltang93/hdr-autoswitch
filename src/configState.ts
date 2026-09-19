import type { ConfigSnapshot, SettingsPatch } from './types.ts';

type Transport = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export interface ConfigView {
  snapshot: ConfigSnapshot | null;
  error: string | null;
  pending: boolean;
}

export type HistoryCommand =
  | 'initialize_config'
  | 'import_legacy_config'
  | 'restore_config'
  | 'reset_config';

export interface MutationOrigin {
  contextToken: string;
  libraryGeneration?: string;
}

export class ConfigClient {
  private transport: Transport;
  private view: ConfigView = { snapshot: null, error: null, pending: false };
  private listeners = new Set<() => void>();
  private readGeneration = 0;
  private tail: Promise<void> = Promise.resolve();
  private pendingCount = 0;
  private authority: Pick<ConfigSnapshot, 'context_token' | 'control_epoch'> | null = null;

  constructor(transport: Transport) {
    this.transport = transport;
  }

  getView = (): ConfigView => this.view;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  private publish(update: Partial<ConfigView>): void {
    this.view = { ...this.view, ...update };
    this.listeners.forEach((listener) => listener());
  }

  reportError = (error: unknown): void => {
    const message = error instanceof Error ? error.message : String(error);
    this.publish({ error: message });
  };

  private accept(next: ConfigSnapshot, allowHistoryChange: boolean): boolean {
    if (this.authority) {
      const epoch = BigInt(next.control_epoch);
      const observedEpoch = BigInt(this.authority.control_epoch);
      if (
        epoch < observedEpoch ||
        (epoch === observedEpoch && next.context_token !== this.authority.context_token)
      ) {
        return false;
      }
    }
    const current = this.view.snapshot;
    if (current) {
      if (next.context_token !== current.context_token) {
        if (!allowHistoryChange || BigInt(next.control_epoch) < BigInt(current.control_epoch)) {
          return false;
        }
      } else if (
        BigInt(next.revision) < BigInt(current.revision) ||
        BigInt(next.control_epoch) < BigInt(current.control_epoch)
      ) {
        return false;
      }
    }
    this.authority = { context_token: next.context_token, control_epoch: next.control_epoch };
    this.publish({ snapshot: next });
    return true;
  }

  private isCurrentContext(context: string): boolean {
    return this.view.snapshot?.context_token === context && this.authority?.context_token === context;
  }

  refresh = async (): Promise<void> => {
    const generation = ++this.readGeneration;
    try {
      const next = await this.transport<ConfigSnapshot>('get_config');
      if (generation === this.readGeneration) this.accept(next, true);
    } catch (error) {
      if (generation === this.readGeneration) this.reportError(error);
    }
  };

  acceptEvent = (next: ConfigSnapshot): void => {
    if (this.authority && BigInt(next.control_epoch) <= BigInt(this.authority.control_epoch)) {
      return;
    }
    const current = this.view.snapshot;
    if (!current || next.context_token !== current.context_token) {
      // Retire the old authority before its asynchronous replacement can be fetched.
      this.authority = { context_token: next.context_token, control_epoch: next.control_epoch };
      void this.refresh();
    } else {
      this.accept(next, false);
    }
  };

  captureOrigin = (): MutationOrigin => {
    const snapshot = this.view.snapshot;
    if (!snapshot || snapshot.mode !== 'ready') {
      throw new Error('Settings are not writable. Resolve the configuration notice first.');
    }
    if (!this.isCurrentContext(snapshot.context_token)) {
      throw new Error('Settings history changed. Wait for the current settings to load.');
    }
    return {
      contextToken: snapshot.context_token,
      libraryGeneration: snapshot.library_generation,
    };
  };

  private enqueue<T>(operation: () => Promise<T>): Promise<T> {
    this.pendingCount += 1;
    this.publish({ pending: true });
    const result = this.tail.then(operation).catch((error: unknown) => {
      this.reportError(error);
      void this.refresh();
      throw error;
    }).finally(() => {
      this.pendingCount -= 1;
      this.publish({ pending: this.pendingCount > 0 });
    });
    // Only the queue tail consumes rejection; the caller still receives the failed result.
    this.tail = result.then(() => undefined, () => undefined);
    return result;
  }

  mutate = (
    command: string,
    args: Record<string, unknown> = {},
    origin?: MutationOrigin,
  ): Promise<ConfigSnapshot> => {
    let captured: MutationOrigin;
    try {
      captured = origin ?? this.captureOrigin();
    } catch (error) {
      this.reportError(error);
      return Promise.reject(error);
    }
    return this.enqueue(async () => {
      if (!this.isCurrentContext(captured.contextToken)) {
        throw new Error('Settings history changed. Start this action again.');
      }
      const next = await this.transport<ConfigSnapshot>(command, {
        ...args,
        expectedContext: captured.contextToken,
        ...(origin?.libraryGeneration !== undefined
          ? { expectedLibraryGeneration: origin.libraryGeneration }
          : {}),
      });
      if (next.context_token !== captured.contextToken || !this.isCurrentContext(captured.contextToken)) {
        throw new Error('This result belongs to retired settings. Review the current settings before trying again.');
      }
      if (this.accept(next, false)) this.publish({ error: null });
      return next;
    });
  };

  patch = (patch: SettingsPatch): Promise<ConfigSnapshot> =>
    this.mutate('patch_settings', { patch });

  changeHistory = (
    command: HistoryCommand,
    args: Record<string, unknown> = {},
  ): Promise<ConfigSnapshot> => {
    const origin = this.view.snapshot?.context_token;
    if (!origin) {
      const error = new Error('Configuration has not loaded.');
      this.reportError(error);
      return Promise.reject(error);
    }
    return this.enqueue(async () => {
      if (!this.isCurrentContext(origin)) {
        throw new Error('Settings history changed. Start this action again.');
      }
      ++this.readGeneration;
      const next = await this.transport<ConfigSnapshot>(command, {
        ...args,
        expectedContext: origin,
      });
      ++this.readGeneration;
      if (!this.accept(next, true) && !this.isCurrentContext(next.context_token)) {
        throw new Error('This result belongs to retired settings. Review the current settings before trying again.');
      }
      this.publish({ error: null });
      return next;
    });
  };
}
