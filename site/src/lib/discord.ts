import { createHash, randomBytes } from "node:crypto";
import { base64urlnopad } from "@scure/base";

export function buildAvatarURL(id: string, avatar: string, discriminator?: string) {
  if (avatar) {
    const extension = avatar.startsWith("a_") ? "gif" : "png";
    return `https://cdn.discordapp.com/avatars/${id}/${avatar}.${extension}?size=256`;
  }

  const index = discriminator && discriminator !== "0" ? Number(discriminator) % 5 : 0;
  return `https://cdn.discordapp.com/embed/avatars/${index}.png`;
}

export function generateCodeVerifier() {
  return base64urlnopad.encode(randomBytes(64));
}

export function deriveCodeChallenge(verifier: string) {
  return base64urlnopad.encode(createHash("sha256").update(verifier, "ascii").digest());
}

export function generateState() {
  return base64urlnopad.encode(randomBytes(16));
}
