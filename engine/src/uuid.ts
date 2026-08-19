/**
 * UUIDv7 — a 48-bit millisecond timestamp, then randomness.
 *
 * §6.5 wants it for the reason it wants event identity at all: **time-ordered, so file order
 * is chronological order with no extra field.** A v4 would need a `createdAt` beside it and
 * would let the two disagree.
 *
 *   unix_ts_ms (48) | ver (4) | rand_a (12) | var (2) | rand_b (62)
 *
 * **`rand_a` is a counter, not randomness, and that is not decoration.** A plain v7 orders
 * two ids minted in the same millisecond by their random tails, which is to say it does not
 * order them — and creating a Character and advancing it happen in the same tick as a matter
 * of course, not as a corner case. RFC 9562 calls this the monotonic method; without it
 * §6.5's *"file order is chronological order with no extra field"* is simply false, and a
 * test caught it on the second event ever minted.
 */
import { randomBytes } from "node:crypto";

let lastMs = -1;
let counter = 0;

export function uuidv7(now: number = Date.now()): string {
  if (now === lastMs) {
    counter = (counter + 1) & 0x0fff;
    // 4,096 ids in one millisecond. Waiting for the clock is the only correct answer left,
    // and it is better than minting an id that sorts wrongly.
    if (counter === 0) return uuidv7(now + 1);
  } else {
    lastMs = now;
    counter = randomBytes(2).readUInt16BE(0) & 0x0fff;
  }
  const b = randomBytes(16);
  // 48 bits of milliseconds, big-endian, into the first six bytes.
  b[0] = (now / 2 ** 40) & 0xff;
  b[1] = (now / 2 ** 32) & 0xff;
  b[2] = (now / 2 ** 24) & 0xff;
  b[3] = (now / 2 ** 16) & 0xff;
  b[4] = (now / 2 ** 8) & 0xff;
  b[5] = now & 0xff;
  b[6] = 0x70 | ((counter >> 8) & 0x0f);   // version 7, then the counter's high nibble
  b[7] = counter & 0xff;                   // …and its low byte: rand_a in full
  b[8] = (b[8]! & 0x3f) | 0x80;   // variant 10
  const h = b.toString("hex");
  return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
}

/** The millisecond a v7 was minted. Only for reading a file back, never for ordering — the
 *  ids sort lexically in the same order, which is the point of using them. */
export function mintedAt(id: string): number {
  return Number.parseInt(id.replaceAll("-", "").slice(0, 12), 16);
}
