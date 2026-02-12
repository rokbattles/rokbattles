import "server-only";
import { z } from "zod";

export const oauthCallbackQuerySchema = z.object({
  code: z.string().min(1).optional(),
  state: z.string().min(1).optional(),
  error: z.string().min(1).optional(),
});

export const discordTokenResponseSchema = z.object({
  access_token: z.string().min(1),
  token_type: z.string().min(1),
  expires_in: z.number().int().nonnegative(),
  refresh_token: z.string().min(1).optional(),
  scope: z.string().min(1),
});

export const discordProfileResponseSchema = z.object({
  id: z.string().min(1),
  username: z.string().min(1),
  global_name: z.string().nullable().optional(),
  discriminator: z.string().min(1),
  avatar: z.string().nullable(),
  email: z.string().email().nullable().optional(),
  verified: z.boolean().optional(),
});

export const oauthStateDocumentSchema = z.object({
  state: z.string().min(1),
  verifier: z.string().min(1),
  createdAt: z.date(),
  expiresAt: z.date(),
});

export const sessionDocumentSchema = z.object({
  sessionId: z.string().min(1),
  userId: z.string().min(1),
  createdAt: z.date(),
  expiresAt: z.date(),
});

export const userDocumentSchema = z.object({
  discordId: z.string().min(1),
  username: z.string().min(1),
  discriminator: z.string().min(1),
  globalName: z.string().nullable(),
  email: z.string().email(),
  avatar: z.string().nullable(),
  createdAt: z.date().optional(),
  updatedAt: z.date().optional(),
});

export type OauthStateDocument = z.infer<typeof oauthStateDocumentSchema>;
export type SessionDocument = z.infer<typeof sessionDocumentSchema>;
export type UserDocument = z.infer<typeof userDocumentSchema>;
