import type { Db } from "mongodb";
import { type NextRequest, NextResponse } from "next/server";
import type { z } from "zod";
import { buildAvatarURL } from "@/lib/auth/discord";
import { rewriteUrl } from "@/lib/auth/rewrite-url";
import {
  discordProfileResponseSchema,
  discordTokenResponseSchema,
  oauthCallbackQuerySchema,
  oauthStateDocumentSchema,
  userDocumentSchema,
} from "@/lib/auth/schemas";
import { createSession } from "@/lib/auth/session";
import clientPromise from "@/lib/mongo";

type RouteResult<T> =
  | {
      ok: true;
      data: T;
    }
  | {
      ok: false;
      response: NextResponse;
    };

interface DiscordEnv {
  clientId: string;
  clientSecret: string;
  redirectUri: string;
}

interface CallbackQuery {
  code: string;
  state: string;
}

type DiscordProfile = z.infer<typeof discordProfileResponseSchema>;

function fail(
  status: number,
  error: string,
  reason?: string
): RouteResult<never> {
  return {
    ok: false,
    response: NextResponse.json(reason ? { error, reason } : { error }, {
      status,
    }),
  };
}

function getDiscordEnv(): RouteResult<DiscordEnv> {
  const clientId = process.env.DISCORD_CLIENT_ID;
  const clientSecret = process.env.DISCORD_CLIENT_SECRET;
  const redirectUri = process.env.DISCORD_REDIRECT_URI;

  if (!(clientId && clientSecret && redirectUri)) {
    return fail(500, "server-misconfigured");
  }

  return {
    ok: true,
    data: {
      clientId,
      clientSecret,
      redirectUri,
    },
  };
}

function parseCallbackQuery(url: URL): RouteResult<CallbackQuery> {
  const queryResult = oauthCallbackQuerySchema.safeParse({
    code: url.searchParams.get("code") ?? undefined,
    state: url.searchParams.get("state") ?? undefined,
    error: url.searchParams.get("error") ?? undefined,
  });

  if (!queryResult.success) {
    return fail(400, "invalid-callback");
  }

  const { code, state, error } = queryResult.data;
  if (error) {
    return fail(400, error);
  }

  if (!(code && state)) {
    return fail(400, "invalid-callback");
  }

  return {
    ok: true,
    data: { code, state },
  };
}

function unwrapFindOneAndDeleteResult(value: unknown) {
  if (value && typeof value === "object" && "value" in value) {
    return (value as { value: unknown }).value;
  }

  return value;
}

async function consumeOauthStateVerifier(
  db: Db,
  state: string
): Promise<RouteResult<string>> {
  const stateLookup = await db.collection("oauthStates").findOneAndDelete({
    state,
    expiresAt: { $gt: new Date() },
  });
  const parsedState = oauthStateDocumentSchema.safeParse(
    unwrapFindOneAndDeleteResult(stateLookup)
  );

  if (!parsedState.success) {
    return fail(400, "invalid-or-expired-state");
  }

  return {
    ok: true,
    data: parsedState.data.verifier,
  };
}

async function exchangeDiscordToken(
  env: DiscordEnv,
  code: string,
  codeVerifier: string
): Promise<RouteResult<string>> {
  const tokenBody = new URLSearchParams();
  tokenBody.set("client_id", env.clientId);
  tokenBody.set("grant_type", "authorization_code");
  tokenBody.set("code", code);
  tokenBody.set("redirect_uri", env.redirectUri);
  tokenBody.set("code_verifier", codeVerifier);

  const basicAuthorization = Buffer.from(
    `${env.clientId}:${env.clientSecret}`
  ).toString("base64");

  const tokenResponse = await fetch("https://discord.com/api/oauth2/token", {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
      Authorization: `Basic ${basicAuthorization}`,
    },
    body: tokenBody,
  });
  if (!tokenResponse.ok) {
    const reason = await tokenResponse.text();
    return fail(400, "token-exchange-failed", reason);
  }

  const tokenPayload = await tokenResponse.json();
  const parsedToken = discordTokenResponseSchema.safeParse(tokenPayload);
  if (!parsedToken.success) {
    return fail(400, "invalid-token-payload");
  }

  return {
    ok: true,
    data: parsedToken.data.access_token,
  };
}

async function fetchDiscordProfile(
  accessToken: string
): Promise<RouteResult<DiscordProfile>> {
  const profileResponse = await fetch("https://discord.com/api/users/@me", {
    headers: {
      Authorization: `Bearer ${accessToken}`,
    },
  });
  if (!profileResponse.ok) {
    return fail(400, "profile-fetch-failed");
  }

  const profilePayload = await profileResponse.json();
  const parsedProfile = discordProfileResponseSchema.safeParse(profilePayload);
  if (!parsedProfile.success) {
    return fail(400, "invalid-profile-payload");
  }

  return {
    ok: true,
    data: parsedProfile.data,
  };
}

export async function GET(req: NextRequest) {
  const forwardedReq = rewriteUrl(req);
  const url = new URL(forwardedReq.url);
  const envResult = getDiscordEnv();
  if (!envResult.ok) {
    return envResult.response;
  }

  const queryResult = parseCallbackQuery(url);
  if (!queryResult.ok) {
    return queryResult.response;
  }

  const client = await clientPromise;
  if (!client) {
    return NextResponse.json(
      { error: "server-misconfigured" },
      { status: 500 }
    );
  }

  const db = client.db();
  const stateResult = await consumeOauthStateVerifier(
    db,
    queryResult.data.state
  );
  if (!stateResult.ok) {
    return stateResult.response;
  }

  const tokenResult = await exchangeDiscordToken(
    envResult.data,
    queryResult.data.code,
    stateResult.data
  );
  if (!tokenResult.ok) {
    return tokenResult.response;
  }

  const profileResult = await fetchDiscordProfile(tokenResult.data);
  if (!profileResult.ok) {
    return profileResult.response;
  }

  const profile = profileResult.data;
  const email = profile.email ?? null;
  const verified = profile.verified ?? false;

  if (!(email && verified)) {
    return NextResponse.json(
      { error: "discord-email-not-verified" },
      { status: 400 }
    );
  }

  const now = new Date();
  const userDocument = userDocumentSchema.parse({
    discordId: profile.id,
    username: profile.username,
    discriminator: profile.discriminator,
    globalName: profile.global_name ?? null,
    email,
    avatar: buildAvatarURL(profile.id, profile.avatar, profile.discriminator),
    updatedAt: now,
  });

  await db.collection("users").updateOne(
    { discordId: userDocument.discordId },
    {
      $set: {
        discordId: userDocument.discordId,
        username: userDocument.username,
        discriminator: userDocument.discriminator,
        globalName: userDocument.globalName,
        email: userDocument.email,
        avatar: userDocument.avatar,
        updatedAt: now,
      },
      $setOnInsert: {
        createdAt: now,
      },
    },
    { upsert: true }
  );

  await createSession(userDocument.discordId);

  return NextResponse.redirect(new URL("/", forwardedReq.url), { status: 302 });
}
