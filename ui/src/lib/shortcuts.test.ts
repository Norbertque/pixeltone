import { describe, expect, it } from 'vitest';

import { type KeyContext, shortcutFor } from './shortcuts';

const FREE: KeyContext = { typing: false, activates: false, slider: false };
const key = (k: string, mods: Partial<{ ctrl: boolean; shift: boolean; alt: boolean; meta: boolean }> = {}) => ({
  key: k,
  ctrlKey: mods.ctrl ?? false,
  shiftKey: mods.shift ?? false,
  altKey: mods.alt ?? false,
  metaKey: mods.meta ?? false,
});

describe('keyboard shortcuts', () => {
  it('maps the transport keys', () => {
    expect(shortcutFor(key(' '), FREE)).toBe('toggle');
    expect(shortcutFor(key('ArrowRight', { ctrl: true }), FREE)).toBe('next');
    expect(shortcutFor(key('ArrowLeft', { ctrl: true }), FREE)).toBe('previous');
    expect(shortcutFor(key('ArrowRight', { shift: true }), FREE)).toBe('seekForward');
    expect(shortcutFor(key('ArrowLeft', { shift: true }), FREE)).toBe('seekBack');
    expect(shortcutFor(key('S', { ctrl: true }), FREE)).toBe('shuffle');
    expect(shortcutFor(key('r', { ctrl: true }), FREE)).toBe('repeat');
    expect(shortcutFor(key('q', { ctrl: true }), FREE)).toBe('queue');
    expect(shortcutFor(key('ArrowUp', { ctrl: true }), FREE)).toBe('volumeUp');
    expect(shortcutFor(key('ArrowDown', { ctrl: true }), FREE)).toBe('volumeDown');
    expect(shortcutFor(key('m', { ctrl: true }), FREE)).toBe('mute');
    expect(shortcutFor(key('?', { shift: true }), FREE)).toBe('help');
    expect(shortcutFor(key('ArrowLeft', { alt: true }), FREE)).toBe('back');
  });

  it('leaves keys to text fields, buttons and sliders', () => {
    const typing = { ...FREE, typing: true };
    expect(shortcutFor(key(' '), typing)).toBeNull();
    expect(shortcutFor(key('ArrowRight', { ctrl: true }), typing)).toBeNull();
    expect(shortcutFor(key('ArrowLeft', { shift: true }), typing)).toBeNull();
    expect(shortcutFor(key('?', { shift: true }), typing)).toBeNull();
    // Ctrl+letter shortcuts don't clash with typing.
    expect(shortcutFor(key('f', { ctrl: true }), typing)).toBe('search');

    expect(shortcutFor(key(' '), { ...FREE, activates: true })).toBeNull();
    expect(shortcutFor(key('ArrowRight', { shift: true }), { ...FREE, slider: true })).toBeNull();
    expect(shortcutFor(key('ArrowRight', { ctrl: true }), { ...FREE, slider: true })).toBeNull();
    expect(shortcutFor(key('ArrowUp', { ctrl: true }), { ...FREE, slider: true })).toBeNull();
    expect(shortcutFor(key('ArrowDown', { ctrl: true }), typing)).toBeNull();
  });

  it('ignores unknown and doubly modified keys', () => {
    expect(shortcutFor(key('x'), FREE)).toBeNull();
    expect(shortcutFor(key('s', { ctrl: true, shift: true }), FREE)).toBeNull();
    expect(shortcutFor(key('s', { meta: true }), FREE)).toBeNull();
    expect(shortcutFor(key('ArrowLeft', { alt: true, ctrl: true }), FREE)).toBeNull();
  });
});
