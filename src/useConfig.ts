import { useSyncExternalStore } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ConfigClient } from './configState';

export const configClient = new ConfigClient(invoke);

export function useConfig() {
  return useSyncExternalStore(configClient.subscribe, configClient.getView);
}
