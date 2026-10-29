import { encode } from 'uqr';
import { describe, expect, it } from 'vitest';

import { qrShape } from './qr';

describe('qrShape', () => {
  it('draws exactly the dark modules', () => {
    const link = 'https://norbertque.github.io/pixeltone/jam/#v=1&h=192.168.1.20&p=47821&s=abc&n=PC';
    const { size, path } = qrShape(link);
    const { data } = encode(link, { ecc: 'M', border: 0 });
    expect(size).toBe(data.length);
    expect(size % 4).toBe(1); // 21, 25, 29…: a QR code's sizes
    let dark = 0;
    for (const m of path.matchAll(/M\d+ \d+h(\d+)/g)) dark += Number(m[1]);
    expect(dark).toBe(data.flat().filter(Boolean).length);
  });
});
