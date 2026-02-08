import "server-only";
import { cookies } from "next/headers";
import { sessionDocumentSchema, userDocumentSchema } from "@/lib/auth/schemas";
import clientPromise from "@/lib/mongo";
import type { CurrentUser } from "@/lib/types/current-user";

export async function fetchCurrentUser(): Promise<CurrentUser | null> {
  const cookieStore = await cookies();
  const sid = cookieStore.get("sid")?.value;
  if (!sid) {
    return null;
  }

  const client = await clientPromise;
  if (!client) {
    return null;
  }

  const db = client.db();
  const sessionCandidate = await db
    .collection("userSessions")
    .findOne({ sessionId: sid });
  const parsedSession = sessionDocumentSchema.safeParse(sessionCandidate);
  if (!parsedSession.success) {
    return null;
  }

  if (parsedSession.data.expiresAt <= new Date()) {
    return null;
  }

  const userCandidate = await db
    .collection("users")
    .findOne({ discordId: parsedSession.data.userId });
  const parsedUser = userDocumentSchema.safeParse(userCandidate);
  if (!parsedUser.success) {
    return null;
  }

  return {
    discordId: parsedUser.data.discordId,
    username: parsedUser.data.username,
    globalName: parsedUser.data.globalName,
    email: parsedUser.data.email,
    avatar: parsedUser.data.avatar,
  };
}
