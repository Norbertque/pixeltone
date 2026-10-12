import { describe, expect, it } from 'vitest';

import { queuedMessage } from './library.svelte';
import { parentDir } from './menus';

describe('menus', () => {
  it('finds the folder of Windows and POSIX paths', () => {
    expect(parentDir('C:\\Music\\Album\\01.flac')).toBe('C:\\Music\\Album');
    expect(parentDir('/music/album/01.flac')).toBe('/music/album');
    expect(parentDir('file.flac')).toBe('file.flac');
  });

  it('confirms what was queued', () => {
    expect(queuedMessage(['Song'], true)).toBe('“Song” will play next');
    expect(queuedMessage(['a', 'b', 'c'], false)).toBe('Added 3 songs to the queue');
    expect(queuedMessage(['a', 'b'], true, 'Album')).toBe('“Album” will play next');
  });
});
