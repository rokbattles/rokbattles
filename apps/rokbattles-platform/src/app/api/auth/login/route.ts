import { NextResponse } from "next/server";
import {
  deriveCodeChallenge,
  generateCodeVerifier,
  generateState,
} from "@/lib/auth/discord";
import { oauthStateDocumentSchema } from "@/lib/auth/schemas";
import clientPromise from "@/lib/mongo";

export async function GET() {
  const discordClientId = process.env.DISCORD_CLIENT_ID;
  const discordRedirectUri = process.env.DISCORD_REDIRECT_URI;
  if (!(discordClientId && discordRedirectUri)) {
    return NextResponse.json(
      { error: "server-misconfigured" },
      { status: 500 }
    );
  }

  const client = await clientPromise;
  if (!client) {
    return NextResponse.json(
      { error: "server-misconfigured" },
      { status: 500 }
    );
  }

  const db = client.db();

  const verifier = generateCodeVerifier();
  const challenge = deriveCodeChallenge(verifier);
  const state = generateState();
  const now = new Date();
  const expiresAt = new Date(now.getTime() + 10 * 60 * 1000);
  const oauthState = oauthStateDocumentSchema.parse({
    state,
    verifier,
    createdAt: now,
    expiresAt,
  });

  await db.collection("oauthStates").insertOne(oauthState);

  const url = new URL("https://discord.com/oauth2/authorize");
  url.searchParams.set("client_id", discordClientId);
  url.searchParams.set("response_type", "code");
  url.searchParams.set("redirect_uri", discordRedirectUri);
  url.searchParams.set("scope", "identify email");
  url.searchParams.set("code_challenge", challenge);
  url.searchParams.set("code_challenge_method", "S256");
  url.searchParams.set("state", state);

  return NextResponse.redirect(url.toString(), { status: 302 });
}
