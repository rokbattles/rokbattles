import { randomBytes } from "node:crypto";
import { base64urlnopad } from "@scure/base";
import { cookies } from "next/headers";
import client from "@/lib/mongo";

function generateSessionId() {
  return base64urlnopad.encode(randomBytes(48));
}

export async function createSession(userId: string) {
  const mongo = await client.connect();
  const db = mongo.db();

  const sid = generateSessionId();
  const now = new Date();
  // 7 days
  const expiresAt = new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000);

  await db.collection("userSessions").insertOne({
    sessionId: sid,
    userId,
    createdAt: now,
    expiresAt,
  });

  const cookieStore = await cookies();
  cookieStore.set("sid", sid, {
    httpOnly: true,
    sameSite: "lax",
    secure: true,
    path: "/",
    expires: expiresAt,
  });
}

export async function deleteSession() {
  const cookieStore = await cookies();
  if (!cookieStore.has("sid")) return;

  const sid = cookieStore.get("sid");
  const mongo = await client.connect();
  const db = mongo.db();

  await db.collection("userSessions").deleteOne({ sessionId: sid });
  cookieStore.delete("sid");
}
