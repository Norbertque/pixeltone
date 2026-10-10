import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

import {
  AUDIO_EXTENSIONS,
  type AppInfo,
  type AppSettings,
  type Backend,
  type DeviceCapabilities,
  type DeviceInfo,
  type HardwareVolumeInfo,
  type PlayerError,
  type PlayerSnapshot,
} from './types';

export const tauriBackend: Backend = {
  appInfo: () => invoke<AppInfo>('app_info'),
  setWindowTheme: (caption, dark) => invoke('set_window_theme', { caption, dark }),

  playerState: () => invoke<PlayerSnapshot>('player_state'),
  playFiles: (paths, start) => invoke('player_play_files', { paths, start }),
  playerAction: (action) => invoke('player_action', { action }),
  seek: (ms) => invoke('player_seek', { ms }),
  setRepeat: (mode) => invoke('player_set_repeat', { mode }),
  async pickAudioFiles() {
    const picked = await open({
      multiple: true,
      directory: false,
      title: 'Play files',
      filters: [{ name: 'Audio', extensions: AUDIO_EXTENSIONS }],
    });
    if (!picked) return [];
    return Array.isArray(picked) ? picked : [picked];
  },
  coverUrl: (key) => convertFileSrc(key, 'cover'),

  outputDevices: () => invoke<DeviceInfo[]>('output_devices'),
  outputCapabilities: (deviceId) => invoke<DeviceCapabilities>('output_capabilities', { deviceId }),
  getSettings: () => invoke<AppSettings>('get_settings'),
  setOutputSettings: (output, prefs) => invoke<AppSettings>('set_output_settings', { output, prefs }),
  hardwareVolume: (deviceId) => invoke<HardwareVolumeInfo | null>('hardware_volume', { deviceId }),
  setHardwareVolume: (deviceId, db) => invoke('set_hardware_volume', { deviceId, db }),

  onPlayerState: (cb) => listen<PlayerSnapshot>('player://state', (e) => cb(e.payload)),
  onPosition: (cb) => listen<{ positionMs: number }>('player://position', (e) => cb(e.payload.positionMs)),
  onPlayerError: (cb) => listen<PlayerError>('player://error', (e) => cb(e.payload)),
};
