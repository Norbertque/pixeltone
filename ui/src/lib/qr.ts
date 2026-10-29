import { encode } from 'uqr';

/** A QR code as one SVG path in a `size`×`size` box (no quiet zone: leave room around it). */
export interface QrShape {
  size: number;
  path: string;
}

export function qrShape(text: string): QrShape {
  const { size, data } = encode(text, { ecc: 'M', border: 0 });
  let path = '';
  data.forEach((row, y) => {
    // Dark runs along each row, as rectangles.
    let x = 0;
    while (x < size) {
      if (!row[x]) {
        x += 1;
        continue;
      }
      const start = x;
      while (x < size && row[x]) x += 1;
      path += `M${start} ${y}h${x - start}v1h${start - x}z`;
    }
  });
  return { size, path };
}
