import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { ConfigClient } from '../src/configState.ts';
import {
  detectionKey, importSelectedDetections, selectDetections, selectedDetections, toggleDetection,
} from '../src/scanSelection.ts';

const game = (launcher, path, enabled = true) => ({
  name: 'Shared game', exe_name: 'game.exe', path, launcher, enabled, hdr_type: 'native',
});
const steam = game('Steam', 'C:\\SteamLibrary\\game.exe');
const gog = game('GOG', 'D:\\GOG\\game.exe');

test('same basename detections have independent provider/path identity, not vector-index identity', () => {
  assert.notEqual(detectionKey(steam), detectionKey(gog));
  assert.notEqual(detectionKey(steam), detectionKey({ ...steam, path: 'E:\\SteamLibrary\\game.exe' }));
  assert.notEqual(detectionKey(steam), detectionKey({ ...steam, launcher: 'GOG' }));
  assert.equal(detectionKey(steam), detectionKey({
    ...steam, exe_name: 'GAME.EXE', launcher: 'STEAM', path: 'c:/steamlibrary/GAME.exe',
  }));
  assert.equal(detectionKey(steam), detectionKey({ ...steam, name: 'A changed display title' }));
  assert.equal(detectionKey(steam), detectionKey({ ...steam, path: '\\\\?\\C:\\SteamLibrary\\game.exe' }));
});

test('actual scan selection handlers keep initial, HDR/all, individual toggle and submission in agreement', () => {
  const games = [steam, { ...gog, enabled: false }];
  const initial = selectDetections(games, (item) => item.enabled);
  assert.deepEqual(selectedDetections(games, initial), [steam]);
  const both = toggleDetection(initial, gog);
  assert.deepEqual(selectedDetections(games, both), [steam, gog]);
  const onlyGog = toggleDetection(both, steam);
  assert.deepEqual(selectedDetections(games, onlyGog), [gog]);
  assert.deepEqual(selectedDetections([...games].reverse(), onlyGog), [gog]);
  assert.deepEqual(selectedDetections(games, selectDetections(games, () => true)), [steam, gog]);
  assert.deepEqual(selectedDetections(games, {}), []);
});

test('real import handler submits only the individually selected installation with original scan fences', async () => {
  let snapshot = {
    mode: 'ready', context_token: 'ctx', library_generation: '7', revision: '2', control_epoch: '2',
    store_id: 'store', settings: { apps: [] },
  };
  const requests = [];
  const client = new ConfigClient(async (command, args) => {
    if (command === 'get_config') return structuredClone(snapshot);
    requests.push({ command, args });
    snapshot = { ...snapshot, revision: '3', control_epoch: '3', library_generation: '8',
      settings: { apps: args.detected } };
    return structuredClone(snapshot);
  });
  await client.refresh();
  const origin = client.captureOrigin();
  const selected = toggleDetection({}, gog);
  assert.equal(await importSelectedDetections(client, [steam, gog], selected, origin), 1);
  assert.equal(requests.length, 1);
  assert.equal(requests[0].command, 'import_detected_games');
  assert.deepEqual(requests[0].args.detected, [gog]);
  assert.equal(requests[0].args.expectedContext, 'ctx');
  assert.equal(requests[0].args.expectedLibraryGeneration, '7');
  assert.deepEqual(client.getView().snapshot.settings.apps, [gog]);
});

test('real import handler leaves selection and settings intact when a conflicting batch is rejected', async () => {
  const snapshot = { mode: 'ready', context_token: 'ctx', library_generation: '7', revision: '2',
    control_epoch: '2', store_id: 'store', settings: { apps: [steam] } };
  let sent;
  const client = new ConfigClient(async (command, args) => {
    if (command === 'get_config') return structuredClone(snapshot);
    sent = args.detected;
    throw new Error('Select only one installation for game.exe.');
  });
  await client.refresh();
  const selected = selectDetections([steam, gog], () => true);
  await assert.rejects(importSelectedDetections(
    client, [steam, gog], selected, client.captureOrigin(),
  ), /Select only one installation/);
  assert.deepEqual(sent, [steam, gog]);
  assert.deepEqual(selectedDetections([steam, gog], selected), [steam, gog]);
  assert.deepEqual(client.getView().snapshot.settings.apps, [steam]);
});

test('both rendered scan sections and confirmation use the tested detection identity handlers', () => {
  const component = readFileSync(new URL('../src/components/AppsManager.tsx', import.meta.url), 'utf8');
  assert.equal((component.match(/key=\{detectionKey\(game\)\}/g) ?? []).length, 2);
  assert.equal((component.match(/toggleDetection\(prev, game\)/g) ?? []).length, 2);
  assert.doesNotMatch(component, /selectedToImport\[(?:game|g)\.exe_name\]/);
  assert.match(component, /importSelectedDetections\(configClient, scannedGames, selectedToImport, scanOrigin\)/);
});
