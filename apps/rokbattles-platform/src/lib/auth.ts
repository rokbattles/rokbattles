import type { Db } from "mongodb";
import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import clientPromise from "@/lib/mongo";
import type { SessionDocument, UserDocument } from "@/lib/types/auth";

export type AuthenticationFailureReason =
  | "missing-cookie"
  | "invalid-session"
  | "session-expired"
  | "user-not-found";

export type AuthenticationResult =
  | {
      ok: true;
      context: AuthenticatedRequestContext;
    }
  | {
      ok: false;
      reason: AuthenticationFailureReason;
    };

export type AuthenticatedRequestContext = {
  sid: string;
  session: SessionDocument;
  user: UserDocument;
  db: Db;
};

export async function authenticateRequest(): Promise<AuthenticationResult> {
  const cookieStore = await cookies();
  const sid = cookieStore.get("sid")?.value;

  if (!sid) {
    return {
      ok: false,
      reason: "missing-cookie",
    };
  }

  const mongo = await clientPromise;
  const db = mongo.db();

  const session = await db
    .collection<SessionDocument>("userSessions")
    .findOne({ sessionId: sid });
  if (!session) {
    cookieStore.delete("sid");
    return {
      ok: false,
      reason: "invalid-session",
    };
  }

  const now = new Date();
  if (session.expiresAt <= now) {
    await db
      .collection<SessionDocument>("userSessions")
      .deleteOne({ sessionId: sid });
    cookieStore.delete("sid");

    return {
      ok: false,
      reason: "session-expired",
    };
  }

  const user = await db
    .collection<UserDocument>("users")
    .findOne({ discordId: session.userId });
  if (!user) {
    await db
      .collection<SessionDocument>("userSessions")
      .deleteOne({ sessionId: sid });
    cookieStore.delete("sid");

    return {
      ok: false,
      reason: "user-not-found",
    };
  }

  return {
    ok: true,
    context: {
      sid,
      session,
      user,
      db,
    },
  };
}

export async function requireAuthContext() {
  const result = await authenticateRequest();

  if (result.ok === true) {
    return { ok: true as const, context: result.context };
  }

  return {
    ok: false as const,
    reason: result.reason,
    response: NextResponse.json(
      { error: "unauthorized", reason: result.reason },
      { status: 401 }
    ),
  };
}
