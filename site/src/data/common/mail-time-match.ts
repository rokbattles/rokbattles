import "server-only";

import type { Document } from "mongodb";

export function buildMailTimeMatch(startMillis: number, endMillis: number): Document {
  const startSeconds = Math.floor(startMillis / 1000);
  const endSeconds = Math.floor(endMillis / 1000);
  const startMicros = Math.floor(startMillis * 1000);
  const endMicros = Math.floor(endMillis * 1000);

  return {
    $or: [
      { "metadata.mail_time": { $gte: startSeconds, $lt: endSeconds } },
      { "metadata.mail_time": { $gte: startMillis, $lt: endMillis } },
      { "metadata.mail_time": { $gte: startMicros, $lt: endMicros } },
    ],
  };
}
