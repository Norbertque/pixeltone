import { describe, expect, it } from 'vitest';

import type { EqSettings } from './api';
import { applyPreset, BANDS, bandLabel, CUSTOM, MAX_DB, PRESETS, setBand } from './eq';

const FLAT: EqSettings = { enabled: true, preset: 'Flat', gainsDb: Array<number>(10).fill(0) };

describe('equaliser', () => {
  it('has a gain for every band in every preset, within range', () => {
    for (const p of PRESETS) {
      expect(p.gains).toHaveLength(BANDS.length);
      expect(p.gains.every((g) => Math.abs(g) <= MAX_DB)).toBe(true);
    }
  });

  it('labels bands', () => {
    expect(BANDS.map(bandLabel)).toEqual(['31', '62', '125', '250', '500', '1k', '2k', '4k', '8k', '16k']);
  });

  it('applies presets by name and ignores unknown ones', () => {
    const rock = applyPreset(FLAT, 'Rock');
    expect(rock.preset).toBe('Rock');
    expect(rock.gainsDb[0]).toBe(4);
    expect(rock.enabled).toBe(true);
    expect(applyPreset(FLAT, 'Nope')).toBe(FLAT);
  });

  it('turns custom when a band moves, and back when it matches a preset', () => {
    const moved = setBand(FLAT, 3, 2.26);
    expect(moved.preset).toBe(CUSTOM);
    expect(moved.gainsDb[3]).toBe(2.5);
    expect(setBand(moved, 3, 0).preset).toBe('Flat');
    expect(setBand(FLAT, 0, 40).gainsDb[0]).toBe(MAX_DB);
  });

  it('repairs a short gain list', () => {
    expect(setBand({ ...FLAT, gainsDb: [1] }, 9, 1).gainsDb).toEqual([1, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
  });
});
