import { NextResponse } from "next/server";
import { deriveCodeChallenge, generateCodeVerifier, generateState } from "@/lib/discord";
import clientPromise from "@/lib/mongo";

export async function GET() {
  const verifier = generateCodeVerifier();
  const challenge = deriveCodeChallenge(verifier);
  const state = generateState();

  const now = new Date();
  const expiresAt = new Date(now.getTime() + 10 * 60 * 1000); // 10 minutes

  const mongo = await clientPromise;
  const db = mongo.db();

  await db.collection("oauthStates").insertOne({
    state,
    verifier,
    createdAt: now,
    expiresAt,
  });

  const url = new URL("https://discord.com/oauth2/authorize");
  url.searchParams.set("client_id", process.env.DISCORD_CLIENT_ID);
  url.searchParams.set("response_type", "code");
  url.searchParams.set("redirect_uri", process.env.DISCORD_REDIRECT_URI);
  url.searchParams.set("scope", "identify email");
  url.searchParams.set("code_challenge", challenge);
  url.searchParams.set("code_challenge_method", "S256");
  url.searchParams.set("state", state);

  return NextResponse.redirect(url.toString(), { status: 302 });
}
