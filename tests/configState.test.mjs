import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ConfigClient } from '../src/configState.ts';

const snapshot = (overrides = {}) => ({
  settings: { apps: [], target_monitor: { kind: 'all' } },
  mode: 'ready',
  store_id: 'history-1',
  revision: '1',
  context_token: 'context-1',
  library_generation: '1',
  control_epoch: '1',
  issue: null,
  controller_issue: null,
  candidates: [],
  config_path: 'isolated-test-config',
  ...overrides,
});

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((ok, fail) => { resolve = ok; reject = fail; });
  return { promise, resolve, reject };
}

test('failed bootstrap never exposes writable defaults', async () => {
  const client = new ConfigClient(async () => { throw new Error('unreadable'); });
  await client.refresh();
  assert.equal(client.getView().snapshot, null);
  await assert.rejects(client.patch({ autostart: true }), /not writable/);
  assert.ok(client.getView().error);
});

test('revisions are compared as decimal integers beyond JS safe integer range', async () => {
  const client = new ConfigClient(async () => snapshot({ revision: '9007199254740993' }));
  await client.refresh();
  client.acceptEvent(snapshot({ revision: '9007199254740992', control_epoch: '2' }));
  assert.equal(client.getView().snapshot.revision, '9007199254740993');
});

test('control gate changes with the same revision cannot be reversed by a stale response', async () => {
  const client = new ConfigClient(async () => snapshot());
  await client.refresh();
  client.acceptEvent(snapshot({ control_epoch: '3', controller_issue: 'conflict' }));
  client.acceptEvent(snapshot({ control_epoch: '2' }));
  assert.equal(client.getView().snapshot.controller_issue, 'conflict');
});

test('queued patches preserve user order and never send an entire configuration', async () => {
  const first = deferred();
  const calls = [];
  const client = new ConfigClient(async (command, args) => {
    if (command === 'get_config') return snapshot();
    calls.push(args);
    if (calls.length === 1) return first.promise;
    return snapshot({ revision: '3', control_epoch: '3' });
  });
  await client.refresh();
  const one = client.patch({ start_minimized: true });
  const two = client.patch({ notifications_enabled: false });
  await Promise.resolve();
  assert.equal(calls.length, 1);
  first.resolve(snapshot({ revision: '2', control_epoch: '2' }));
  await Promise.all([one, two]);
  assert.deepEqual(calls.map((call) => call.patch), [
    { start_minimized: true }, { notifications_enabled: false },
  ]);
  assert.equal(client.getView().snapshot.revision, '3');
  assert.equal(client.getView().pending, false);
});

test('failed writes keep canonical settings and do not poison the mutation queue', async () => {
  let calls = 0;
  const client = new ConfigClient(async (command) => {
    if (command === 'get_config') return snapshot();
    if (++calls === 1) throw new Error('disk full');
    return snapshot({ revision: '2', control_epoch: '2' });
  });
  await client.refresh();
  await assert.rejects(client.patch({ autostart: true }), /disk full/);
  assert.equal(client.getView().snapshot.revision, '1');
  assert.equal(client.getView().error, 'disk full');
  await client.patch({ notifications_enabled: false });
  assert.equal(client.getView().snapshot.revision, '2');
});

test('a committed reset fences queued old-history work and an outstanding refetch', async () => {
  const oldRead = deferred();
  const reset = deferred();
  let reads = 0;
  let patches = 0;
  const client = new ConfigClient(async (command) => {
    if (command === 'get_config') return ++reads === 2 ? oldRead.promise : snapshot();
    if (command === 'reset_config') return reset.promise;
    patches += 1;
    return snapshot();
  });
  await client.refresh();
  const refetch = client.refresh();
  const transition = client.changeHistory('reset_config');
  const stale = client.patch({ autostart: true });
  const rejection = assert.rejects(stale, /history changed/);
  reset.resolve(snapshot({
    store_id: 'history-2', context_token: 'context-2', control_epoch: '10',
  }));
  await transition;
  oldRead.resolve(snapshot({ revision: '99', control_epoch: '9' }));
  await refetch;
  await rejection;
  assert.equal(client.getView().snapshot.store_id, 'history-2');
  assert.equal(patches, 0);
});

test('a history-change event retires an outstanding bootstrap', async () => {
  const oldRead = deferred();
  let reads = 0;
  const current = snapshot({ store_id: 'new', context_token: 'new', control_epoch: '5' });
  const client = new ConfigClient(async () => ++reads === 1 ? oldRead.promise : current);
  const bootstrap = client.refresh();
  client.acceptEvent(current);
  await Promise.resolve();
  oldRead.resolve(snapshot());
  await bootstrap;
  assert.equal(client.getView().snapshot.store_id, 'new');
});

test('explicit scan origin sends its original library generation', async () => {
  let sent;
  const client = new ConfigClient(async (command, args) => {
    if (command === 'get_config') return snapshot();
    sent = args;
    return snapshot({ revision: '2', control_epoch: '2' });
  });
  await client.refresh();
  const origin = client.captureOrigin();
  client.acceptEvent(snapshot({ revision: '2', library_generation: '2', control_epoch: '2' }));
  await client.mutate('import_detected_games', { detected: [] }, origin);
  assert.equal(sent.expectedLibraryGeneration, '1');
});

test('an in-flight mutation cannot report Saved after its authority is retired', async () => {
  const response = deferred();
  let current = snapshot();
  const client = new ConfigClient(async (command) => command === 'get_config' ? current : response.promise);
  await client.refresh();
  const mutation = client.patch({ start_minimized: true });
  const rejected = assert.rejects(mutation, /retired settings/);
  await Promise.resolve();
  current = snapshot({ context_token: 'recovery-context', mode: 'recovery_required', control_epoch: '5' });
  client.acceptEvent(current);
  await Promise.resolve();
  response.resolve(snapshot({ revision: '2', control_epoch: '2' }));
  await rejected;
  assert.equal(client.getView().snapshot.context_token, 'recovery-context');
  assert.equal(client.getView().snapshot.mode, 'recovery_required');
});

test('a newer authority fences in-flight and queued work before its refetch completes', async () => {
  const response = deferred();
  const canonical = deferred();
  const next = snapshot({ store_id: 'history-2', context_token: 'context-2', control_epoch: '10' });
  let reads = 0;
  const calls = [];
  const client = new ConfigClient(async (command, args) => {
    if (command === 'get_config') return ++reads === 1 ? snapshot() : canonical.promise;
    calls.push({ command, args });
    return calls.length === 1 ? response.promise : { ...next, revision: '2', control_epoch: '11' };
  });
  await client.refresh();
  const inFlight = client.patch({ start_minimized: true });
  const queuedPatch = client.patch({ autostart: true });
  const queuedReset = client.changeHistory('reset_config');
  const rejected = [
    assert.rejects(inFlight, /retired settings/),
    assert.rejects(queuedPatch, /history changed/),
    assert.rejects(queuedReset, /history changed/),
  ];
  await Promise.resolve();
  client.acceptEvent(next);
  assert.throws(client.captureOrigin, /history changed/);
  response.resolve(snapshot({ revision: '2', control_epoch: '2' }));
  await Promise.all(rejected);
  assert.deepEqual(calls.map((call) => call.command), ['patch_settings']);
  assert.equal(client.getView().snapshot.context_token, 'context-1');
  assert.equal(client.getView().snapshot.revision, '1');

  canonical.resolve(next);
  await client.refresh();
  assert.equal(client.captureOrigin().contextToken, 'context-2');
  await client.patch({ start_minimized: false });
  assert.equal(calls[1].args.expectedContext, 'context-2');
});

test('delayed older-context events cannot retire the current authority or trigger a refetch', async () => {
  const current = snapshot({ context_token: 'current-context', control_epoch: '9007199254740993' });
  let reads = 0;
  const client = new ConfigClient(async (command, args) => {
    if (command === 'get_config') {
      reads += 1;
      return current;
    }
    assert.equal(args.expectedContext, 'current-context');
    return { ...current, revision: '2', control_epoch: '9007199254740994' };
  });
  await client.refresh();
  client.acceptEvent(snapshot({ control_epoch: '9007199254740992' }));
  client.acceptEvent(snapshot({ control_epoch: '9007199254740993' }));
  assert.equal(reads, 1);
  assert.equal(client.captureOrigin().contextToken, 'current-context');
  await client.patch({ autostart: true });
  assert.equal(client.getView().snapshot.control_epoch, '9007199254740994');
});

test('only the newest announced authority can finish reconciliation', async () => {
  const intermediate = deferred();
  const latest = deferred();
  const second = snapshot({ context_token: 'context-2', control_epoch: '10' });
  const third = snapshot({ context_token: 'context-3', control_epoch: '20' });
  const responses = [snapshot(), intermediate.promise, latest.promise];
  let reads = 0;
  const client = new ConfigClient(async () => responses[reads++]);
  await client.refresh();
  client.acceptEvent(second);
  client.acceptEvent(third);
  client.acceptEvent(second);
  client.acceptEvent(snapshot({ control_epoch: '9' }));
  assert.equal(reads, 3);
  assert.throws(client.captureOrigin, /history changed/);

  latest.resolve(third);
  await new Promise(setImmediate);
  assert.equal(client.getView().snapshot.context_token, 'context-3');
  intermediate.resolve(second);
  await new Promise(setImmediate);
  assert.equal(client.getView().snapshot.context_token, 'context-3');
  assert.equal(client.captureOrigin().contextToken, 'context-3');
});

test('a history command can reconcile its own announced context before the refetch returns', async () => {
  const response = deferred();
  const canonical = deferred();
  const next = snapshot({ store_id: 'history-2', context_token: 'context-2', control_epoch: '10' });
  let reads = 0;
  const client = new ConfigClient(async (command) => {
    if (command === 'get_config') return ++reads === 1 ? snapshot() : canonical.promise;
    return response.promise;
  });
  await client.refresh();
  const transition = client.changeHistory('reset_config');
  await Promise.resolve();
  client.acceptEvent(next);
  response.resolve(next);
  await transition;
  assert.equal(client.getView().snapshot.store_id, 'history-2');
  assert.equal(client.captureOrigin().contextToken, 'context-2');
  canonical.resolve(snapshot({ revision: '2', control_epoch: '2' }));
  await new Promise(setImmediate);
  assert.equal(client.getView().snapshot.store_id, 'history-2');
});

test('failed reconciliation and stale reads never restore retired authority', async () => {
  const next = snapshot({ context_token: 'context-2', control_epoch: '10' });
  let result = snapshot();
  let failRead = false;
  const client = new ConfigClient(async () => {
    if (failRead) throw new Error('Refetch failed');
    return result;
  });
  await client.refresh();
  failRead = true;
  client.acceptEvent(next);
  await new Promise(setImmediate);
  assert.equal(client.getView().error, 'Refetch failed');
  assert.throws(client.captureOrigin, /history changed/);
  failRead = false;
  result = snapshot({ revision: '2', control_epoch: '2' });
  await client.refresh();
  assert.equal(client.getView().snapshot.revision, '1');
  assert.throws(client.captureOrigin, /history changed/);
  result = next;
  await client.refresh();
  assert.equal(client.captureOrigin().contextToken, 'context-2');
});
