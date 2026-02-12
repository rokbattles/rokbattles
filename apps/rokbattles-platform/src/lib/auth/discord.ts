import "server-only";
import { createHash, randomBytes } from "node:crypto";

function base64UrlNoPadding(buffer: Buffer) {
  return buffer.toString("base64url");
}

export function buildAvatarURL(
  id: string,
  avatar: string | null,
  discriminator?: string
) {
  if (avatar) {
    const extension = avatar.startsWith("a_") ? "gif" : "png";
    return `https://cdn.discordapp.com/avatars/${id}/${avatar}.${extension}`;
  }

  const index =
    discriminator && discriminator !== "0" ? Number(discriminator) % 5 : 0;
  return `https://cdn.discordapp.com/embed/avatars/${index}.png`;
}

export function generateCodeVerifier() {
  return base64UrlNoPadding(randomBytes(64));
}

export function deriveCodeChallenge(verifier: string) {
  return base64UrlNoPadding(
    createHash("sha256").update(verifier, "ascii").digest()
  );
}

export function generateState() {
  return base64UrlNoPadding(randomBytes(16));
}
