import type { Session } from "@synonymdev/pubky";

/**
 * Generate a timestamp ID (Crockford Base32 of unix microseconds).
 * Matches the Rust indexer's expected format.
 */
function generateTimestampId(): string {
  const CROCKFORD = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";
  const now = BigInt(Date.now()) * 1000n; // milliseconds to microseconds
  const bytes = new ArrayBuffer(8);
  const view = new DataView(bytes);
  view.setBigInt64(0, now, false); // big-endian

  const byteArray = new Uint8Array(bytes);
  let bits = 0n;
  for (const b of byteArray) {
    bits = (bits << 8n) | BigInt(b);
  }

  let result = "";
  // 8 bytes = 64 bits, ceil(64/5) = 13 chars
  for (let i = 12; i >= 0; i--) {
    const idx = Number((bits >> (BigInt(i) * 5n)) & 0x1fn);
    result += CROCKFORD[idx];
  }

  return result;
}

/**
 * Write a pixel event to the user's homeserver.
 */
export async function placePixel(
  session: Session,
  x: number,
  y: number,
  color: number
): Promise<string> {
  const id = generateTimestampId();
  const path = `/pub/pubky-canva/pixels/${id}` as `/pub/${string}`;
  const data = { x, y, color };

  await session.storage.putJson(path, data);
  return id;
}
