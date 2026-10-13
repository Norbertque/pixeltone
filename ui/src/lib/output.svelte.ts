/**
 * Output settings shared by Settings and the player bar's volume button: the saved
 * settings, the selected device's volume control, and the volume currently in charge.
 *
 * Changes are saved one after another, each on top of the last saved state, so a quick
 * series (a slider being dragged, a switch flipped meanwhile) can't lose any of them.
 */
import {
  api,
  type AppSettings,
  type DeviceCapabilities,
  type DevicePrefs,
  type EqSettings,
  type HardwareVolumeInfo,
  type OutputSettings,
} from './api';
import { player } from './player.svelte';
import { toasts } from './toasts.svelte';
import { devicePrefs, FLOOR_DB, volumeControl, type VolumeControl } from './volume';

class Output {
  settings = $state<AppSettings | null>(null);
  /** The selected device's own volume control; `null` when it has none. */
  hardware = $state<HardwareVolumeInfo | null>(null);
  caps = $state<DeviceCapabilities | null>(null);
  capsLoading = $state(false);
  /** The volume being dragged to, ahead of the device or the saved settings. */
  draft = $state<number | null>(null);
  /** Equaliser gains being dragged, ahead of the saved settings. */
  eqDraft = $state<EqSettings | null>(null);

  readonly deviceId = $derived(this.settings?.output.deviceId ?? null);
  readonly prefs = $derived<DevicePrefs | null>(this.settings ? devicePrefs(this.settings) : null);
  readonly volume = $derived<VolumeControl | null>(
    this.settings ? volumeControl(this.settings, this.hardware, player.snapshot?.output?.active ?? null) : null,
  );
  readonly eq = $derived<EqSettings | null>(this.eqDraft ?? this.settings?.output.eq ?? null);

  #started: Promise<void> | null = null;
  #chain: Promise<void> = Promise.resolve();
  /** The newest job per slider, waiting for its turn in the chain. */
  #latest = new Map<string, () => Promise<void>>();
  /** Level to return to when unmuting the app's own volume. */
  #beforeMute = -20;

  /** Loads the settings once and follows changes from then on. */
  start(): Promise<void> {
    this.#started ??= (async () => {
      void api.onSettingsChanged((s) => (this.settings = s));
      let timer: ReturnType<typeof setTimeout> | undefined;
      // Device notifications come in bursts (one per role and state): let them settle.
      void api.onDevicesChanged(() => {
        clearTimeout(timer);
        timer = setTimeout(() => void this.refreshHardware(), 400);
      });
      try {
        this.settings = await api.getSettings();
      } catch (e) {
        toasts.show(`Can't read the settings: ${e}`, 'error');
      }
      await this.refreshHardware();
    })();
    return this.#started;
  }

  async refreshHardware(): Promise<void> {
    try {
      this.hardware = await api.hardwareVolume(this.deviceId);
    } catch {
      this.hardware = null;
    }
  }

  /** What the selected device can do in exclusive mode (Settings shows it). */
  async probe(): Promise<void> {
    this.capsLoading = true;
    try {
      this.caps = await api.outputCapabilities(this.deviceId);
      await this.refreshHardware();
    } catch (e) {
      this.caps = null;
      toasts.show(`Can't read the device's capabilities: ${e}`, 'error');
    } finally {
      this.capsLoading = false;
    }
  }

  /** Changes output settings and the selected device's preferences. */
  update(patch: Partial<OutputSettings>, prefsPatch: Partial<DevicePrefs> = {}): Promise<void> {
    return this.#enqueue(() => this.#save(patch, prefsPatch));
  }

  /** Sets the volume in charge (see `volume`). */
  setVolume(db: number): void {
    this.draft = db;
    this.#newest('volume', async () => {
      await this.#applyVolume(db);
      if (!this.#latest.has('volume')) this.draft = null;
    });
  }

  /** Changes the equaliser; it applies to the music immediately. */
  setEq(eq: EqSettings): void {
    this.eqDraft = eq;
    this.#newest('eq', async () => {
      await this.#save({ eq }, {});
      if (!this.#latest.has('eq')) this.eqDraft = null;
    });
  }

  toggleMute(): void {
    const v = this.volume;
    if (!v || v.kind === 'fixed') return;
    if (v.kind === 'device') {
      const muted = !v.muted;
      void this.#enqueue(async () => {
        try {
          await api.setHardwareMute(this.deviceId, muted);
          if (this.hardware) this.hardware = { ...this.hardware, muted };
        } catch (e) {
          toasts.show(`Can't mute the device: ${e}`, 'error');
        }
      });
    } else if (v.muted) {
      this.setVolume(Math.min(v.max, this.#beforeMute));
    } else {
      this.#beforeMute = this.draft ?? v.db;
      this.setVolume(FLOOR_DB);
    }
  }

  #enqueue(job: () => Promise<void>): Promise<void> {
    const run = this.#chain.then(job);
    this.#chain = run.catch(() => {});
    return run;
  }

  /** Queues `job` unless one for `key` is already waiting, which it then replaces. */
  #newest(key: string, job: () => Promise<void>): void {
    const waiting = this.#latest.has(key);
    this.#latest.set(key, job);
    if (waiting) return;
    void this.#enqueue(async () => {
      const run = this.#latest.get(key);
      this.#latest.delete(key);
      await run?.();
    });
  }

  async #save(patch: Partial<OutputSettings>, prefsPatch: Partial<DevicePrefs>): Promise<void> {
    const current = this.settings;
    if (!current) return;
    // Plain copies: the settings are reactive proxies, which not every transport can send.
    const next = $state.snapshot({ ...current.output, ...patch });
    const nextPrefs = $state.snapshot({ ...devicePrefs({ ...current, output: next }), ...prefsPatch });
    try {
      this.settings = await api.setOutputSettings(next, nextPrefs);
    } catch (e) {
      toasts.show(`Can't change the output: ${e}`, 'error');
      return;
    }
    if ('deviceId' in patch) void this.probe();
  }

  async #applyVolume(db: number): Promise<void> {
    const v = this.volume;
    switch (v?.kind) {
      case 'app':
        return this.#save({ sharedGainDb: db }, {});
      case 'software':
        return this.#save({}, { softwareGainDb: db });
      case 'device':
        try {
          await api.setHardwareVolume(this.deviceId, db);
          if (this.hardware) this.hardware = { ...this.hardware, db };
        } catch (e) {
          toasts.show(`Can't change the volume: ${e}`, 'error');
        }
    }
  }
}

export const output = new Output();
