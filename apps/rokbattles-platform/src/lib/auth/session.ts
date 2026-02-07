import "server-only";
import { randomBytes } from "node:crypto";
import { cookies } from "next/headers";
import { sessionDocumentSchema } from "@/lib/auth/schemas";
import clientPromise from "@/lib/mongo";

function generateSessionId() {
  return randomBytes(48).toString("base64url");
}

export async function createSession(userId: string) {
  const client = await clientPromise;
  if (!client) {
    throw new Error("MongoDB client unavailable");
  }

  const db = client.db();

  const sid = generateSessionId();
  const now = new Date();
  const expiresAt = new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000);
  const sessionDocument = sessionDocumentSchema.parse({
    sessionId: sid,
    userId,
    createdAt: now,
    expiresAt,
  });

  await db.collection("userSessions").insertOne(sessionDocument);

  const cookieStore = await cookies();
  cookieStore.set("sid", sid, {
    httpOnly: true,
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
    path: "/",
    expires: expiresAt,
  });
}

export async function deleteSession() {
  const cookieStore = await cookies();
  const sid = cookieStore.get("sid")?.value;

  if (sid) {
    const client = await clientPromise;
    if (client) {
      const db = client.db();
      await db.collection("userSessions").deleteOne({ sessionId: sid });
    }
  }

  cookieStore.delete("sid");
}
