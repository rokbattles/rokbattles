import "server-only";
import { cache } from "react";
import { fetchBindsForUser } from "@/lib/bind";
import clientPromise from "@/lib/mongo";
import type { UserBindView } from "@/lib/types/user-bind";

export const fetchCurrentUserBinds = cache(async function fetchCurrentUserBinds(
  discordId: string
): Promise<UserBindView[]> {
  const client = await clientPromise;
  if (!client) {
    return [];
  }

  const docs = await fetchBindsForUser(client.db(), discordId);

  return docs.map((doc) => ({
    id: doc._id.toString(),
    governorId: doc.governorId,
    kingdom: doc.kingdom ?? null,
    appUid: doc.appUid,
    name: doc.name,
    avatarUrl: doc.avatarUrl,
    frameUrl: doc.frameUrl,
    type: doc.type,
    isDefault: doc.isDefault,
    isVisible: doc.isVisible,
    pendingDeleteAt: doc.pendingDeleteAt
      ? doc.pendingDeleteAt.toISOString()
      : null,
    createdAt: doc.createdAt.toISOString(),
    updatedAt: doc.updatedAt.toISOString(),
  }));
});
