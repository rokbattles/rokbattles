import { type NextRequest, NextResponse } from "next/server";
import { buildAvatarURL } from "@/lib/discord";
import clientPromise from "@/lib/mongo";
import { rewriteUrl } from "@/lib/rewrite-url";
import { createSession } from "@/lib/session";

type TokenResponse = {
  access_token: string;
  token_type: string;
  expires_in: number;
  refresh_token?: string;
  scope: string;
};

type ProfileResponse = {
  id: string;
  username: string;
  global_name: string | null;
  discriminator: string;
  avatar: string | null;
  email?: string | null;
  verified?: boolean;
};

export async function GET(req: NextRequest) {
  const forwardedReq = rewriteUrl(req);

  const url = new URL(forwardedReq.url);
  const code = url.searchParams.get("code");
  const state = url.searchParams.get("state");
  const error = url.searchParams.get("error");

  if (error) {
    return NextResponse.json({ error }, { status: 400 });
  }

  if (!code || !state) {
    return NextResponse.json({ error: "Missing code or state" }, { status: 400 });
  }

  const mongo = await clientPromise;
  const db = mongo.db();

  const oauthState = await db.collection("oauthStates").findOne({ state });
  if (!oauthState) {
    return NextResponse.json({ error: "Invalid or expired state" }, { status: 400 });
  }

  const { verifier } = oauthState;

  const tokenBody = new URLSearchParams();
  tokenBody.set("client_id", process.env.DISCORD_CLIENT_ID);
  tokenBody.set("grant_type", "authorization_code");
  tokenBody.set("code", code);
  tokenBody.set("redirect_uri", process.env.DISCORD_REDIRECT_URI);
  tokenBody.set("code_verifier", verifier);

  const basicAuthorization = Buffer.from(
    `${process.env.DISCORD_CLIENT_ID}:${process.env.DISCORD_CLIENT_SECRET}`
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

    await db.collection("oauthStates").deleteOne({ state });

    return NextResponse.json({ error: "Token exchange failed", reason }, { status: 400 });
  }

  const token = (await tokenResponse.json()) as TokenResponse;

  const profileResponse = await fetch("https://discord.com/api/users/@me", {
    headers: {
      Authorization: `Bearer ${token.access_token}`,
    },
  });

  if (!profileResponse.ok) {
    await db.collection("oauthStates").deleteOne({ state });

    return NextResponse.json({ error: "Failed to fetch Discord profile" }, { status: 400 });
  }

  const profile = (await profileResponse.json()) as ProfileResponse;

  const email = profile.email ?? null;
  const verified = profile.verified ?? false;

  if (!email || !verified) {
    await db.collection("oauthStates").deleteOne({ state });

    return NextResponse.json({ error: "Discord email not verified." }, { status: 400 });
  }

  const avatarUrl = buildAvatarURL(profile.id, profile.avatar, profile.discriminator);
  const now = new Date();

  await db.collection("users").updateOne(
    { discordId: profile.id },
    {
      $set: {
        discordId: profile.id,
        username: profile.username,
        discriminator: profile.discriminator,
        globalName: profile.global_name ?? null,
        email,
        avatar: avatarUrl,
        updatedAt: now,
      },
      $setOnInsert: {
        createdAt: now,
      },
    },
    { upsert: true }
  );

  await db.collection("oauthStates").deleteOne({ state });
  await createSession(profile.id);

  return NextResponse.redirect(new URL("/", forwardedReq.url), { status: 302 });
}
