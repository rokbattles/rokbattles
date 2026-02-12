import "server-only";
import { type Db, ObjectId } from "mongodb";
import type {
  UserBindDocument,
  UserBindFields,
  UserBindType,
} from "@/lib/types/user-bind";

interface GovernorSnapshot {
  governorId: number;
  kingdom: number | null;
  appUid: number | null;
  name: string | null;
  avatarUrl: string | null;
  frameUrl: string | null;
}

interface KingdomGovernorProfile {
  name: string | null;
  kingdom: number | null;
}

export interface BindMutationSummary {
  created: number;
  updated: number;
  skipped: number;
}

export interface RefreshBindsSummary {
  processed: number;
  updated: number;
  inserted: number;
  skipped: number;
  errors: number;
}

export class BindServiceError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "BindServiceError";
  }
}

function toGroupKey(appUid: number | null, governorId: number): string {
  return appUid == null ? `governor:${governorId}` : `appUid:${appUid}`;
}

function hasPendingDelete(bind: UserBindDocument): boolean {
  return bind.pendingDeleteAt instanceof Date;
}

function activeFilter(discordId: string) {
  return {
    discordId,
    pendingDeleteAt: { $exists: false },
  };
}

function sanitizeGovernorId(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }

  const id = Math.trunc(value);
  if (id <= 0) {
    return null;
  }

  return id;
}

function sanitizeOptionalNumber(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }

  return Math.trunc(value);
}

function sanitizePositiveInt(value: unknown): number | null {
  const parsed = sanitizeOptionalNumber(value);
  if (parsed == null || parsed <= 0) {
    return null;
  }

  return parsed;
}

function sanitizeOptionalString(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }

  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function snapshotFromParticipant(
  participant: unknown
): GovernorSnapshot | null {
  if (!participant || typeof participant !== "object") {
    return null;
  }

  const row = participant as Record<string, unknown>;
  const governorId = sanitizeGovernorId(row.player_id);
  if (governorId == null) {
    return null;
  }

  return {
    governorId,
    kingdom: null,
    appUid: sanitizeOptionalNumber(row.app_uid),
    name: sanitizeOptionalString(row.player_name),
    avatarUrl: sanitizeOptionalString(row.avatar_url),
    frameUrl: sanitizeOptionalString(row.frame_url),
  };
}

async function lookupKingdomGovernorProfile(
  db: Db,
  governorId: number
): Promise<KingdomGovernorProfile> {
  const governor = await db
    .collection("kingdomGovernorData")
    .find(
      {
        governorId,
      },
      {
        projection: {
          governorName: 1,
          kingdom: 1,
          date: 1,
        },
      }
    )
    .sort({ date: -1, _id: -1 })
    .limit(1)
    .next();

  if (!governor || typeof governor !== "object") {
    return {
      name: null,
      kingdom: null,
    };
  }

  const row = governor as { governorName?: unknown; kingdom?: unknown };
  return {
    name: sanitizeOptionalString(row.governorName),
    kingdom: sanitizePositiveInt(row.kingdom),
  };
}

async function lookupLatestGovernorSnapshot(
  db: Db,
  governorId: number
): Promise<GovernorSnapshot | null> {
  const row = await db
    .collection("mails_battle")
    .aggregate<{
      participant: unknown;
    }>([
      {
        $match: {
          $or: [
            { "sender.player_id": governorId },
            { "opponents.player_id": governorId },
          ],
        },
      },
      {
        $sort: {
          "metadata.mail_time": -1,
          _id: -1,
        },
      },
      {
        $project: {
          participantCandidates: {
            $concatArrays: [["$sender"], { $ifNull: ["$opponents", []] }],
          },
        },
      },
      {
        $unwind: "$participantCandidates",
      },
      {
        $match: {
          "participantCandidates.player_id": governorId,
        },
      },
      {
        $project: {
          _id: 0,
          participant: "$participantCandidates",
        },
      },
      {
        $limit: 1,
      },
    ])
    .next();

  if (!row) {
    return null;
  }

  return snapshotFromParticipant(row.participant);
}

async function lookupAssociatedSnapshotsByAppUid(
  db: Db,
  appUid: number
): Promise<GovernorSnapshot[]> {
  const rows = await db
    .collection("mails_battle")
    .aggregate<{
      participant: unknown;
    }>([
      {
        $match: {
          $or: [{ "sender.app_uid": appUid }, { "opponents.app_uid": appUid }],
        },
      },
      {
        $project: {
          mailTime: "$metadata.mail_time",
          participants: {
            $concatArrays: [["$sender"], { $ifNull: ["$opponents", []] }],
          },
        },
      },
      {
        $unwind: "$participants",
      },
      {
        $match: {
          "participants.app_uid": appUid,
        },
      },
      {
        $sort: {
          mailTime: -1,
          _id: -1,
        },
      },
      {
        $group: {
          _id: "$participants.player_id",
          participant: { $first: "$participants" },
        },
      },
      {
        $project: {
          _id: 0,
          participant: 1,
        },
      },
    ])
    .toArray();

  return rows
    .map((row) => snapshotFromParticipant(row.participant))
    .filter((snapshot): snapshot is GovernorSnapshot => snapshot != null);
}

async function hydrateGovernorProfiles(
  db: Db,
  snapshots: GovernorSnapshot[]
): Promise<GovernorSnapshot[]> {
  const profileCache = new Map<number, KingdomGovernorProfile>();

  const hydrated = await Promise.all(
    snapshots.map(async (snapshot) => {
      const cached = profileCache.get(snapshot.governorId);
      const fallbackProfile =
        cached ?? (await lookupKingdomGovernorProfile(db, snapshot.governorId));
      profileCache.set(snapshot.governorId, fallbackProfile);
      return {
        ...snapshot,
        name: snapshot.name ?? fallbackProfile.name,
        kingdom: fallbackProfile.kingdom,
      };
    })
  );

  return hydrated;
}

function toObjectId(id: string): ObjectId {
  if (!ObjectId.isValid(id)) {
    throw new BindServiceError("Invalid bind id.");
  }

  return new ObjectId(id);
}

export function parseGovernorIdInput(rawGovernorId: unknown): number {
  const parsed =
    typeof rawGovernorId === "string"
      ? Number.parseInt(rawGovernorId, 10)
      : rawGovernorId;

  const governorId = sanitizeGovernorId(parsed);
  if (governorId == null) {
    throw new BindServiceError("Enter a valid governor ID.");
  }

  return governorId;
}

export function fetchBindsForUser(
  db: Db,
  discordId: string
): Promise<UserBindDocument[]> {
  return db
    .collection<UserBindFields>("user_binds")
    .find(
      { discordId },
      {
        projection: {
          discordId: 0,
        },
      }
    )
    .sort({ isDefault: -1, isVisible: -1, updatedAt: -1, createdAt: -1 })
    .toArray();
}

// biome-ignore lint/complexity/noExcessiveCognitiveComplexity: This flow validates limits and merges multiple discovery sources in one transaction-like path.
export async function createBindsFromGovernor(
  db: Db,
  discordId: string,
  governorId: number
): Promise<BindMutationSummary> {
  const bindsCollection = db.collection<UserBindFields>("user_binds");
  const now = new Date();

  const existingGovernorBind = await bindsCollection.findOne({ governorId });
  if (existingGovernorBind && existingGovernorBind.discordId !== discordId) {
    throw new BindServiceError(
      "That governor is already bound by another account."
    );
  }

  const activeBinds = await bindsCollection
    .find(activeFilter(discordId))
    .toArray();

  const activeGroups = new Set(
    activeBinds.map((bind) => toGroupKey(bind.appUid ?? null, bind.governorId))
  );

  const activeVisibleCount = activeBinds.filter(
    (bind) => bind.isVisible
  ).length;
  const activeDefault = activeBinds.find((bind) => bind.isDefault) ?? null;

  const latestSnapshot = await lookupLatestGovernorSnapshot(db, governorId);
  const entrySnapshotBase: GovernorSnapshot = latestSnapshot ?? {
    governorId,
    kingdom: null,
    appUid: null,
    name: null,
    avatarUrl: null,
    frameUrl: null,
  };

  const [entrySnapshot] = await hydrateGovernorProfiles(db, [
    entrySnapshotBase,
  ]);

  const candidateSnapshotsRaw =
    entrySnapshot.appUid == null
      ? [entrySnapshot]
      : await lookupAssociatedSnapshotsByAppUid(db, entrySnapshot.appUid);

  const candidateSnapshotsMap = new Map<number, GovernorSnapshot>();
  for (const snapshot of candidateSnapshotsRaw) {
    candidateSnapshotsMap.set(snapshot.governorId, snapshot);
  }

  if (!candidateSnapshotsMap.has(governorId)) {
    candidateSnapshotsMap.set(governorId, entrySnapshot);
  }

  const candidateSnapshots = await hydrateGovernorProfiles(
    db,
    Array.from(candidateSnapshotsMap.values())
  );

  const incomingGroupKey = toGroupKey(entrySnapshot.appUid, governorId);
  if (!activeGroups.has(incomingGroupKey) && activeGroups.size >= 3) {
    throw new BindServiceError("You can only bind up to 3 account groups.");
  }

  const candidateGovernorIds = candidateSnapshots.map(
    (snapshot) => snapshot.governorId
  );
  const existingCandidateBinds = await bindsCollection
    .find({ governorId: { $in: candidateGovernorIds } })
    .toArray();
  const existingByGovernorId = new Map(
    existingCandidateBinds.map((bind) => [bind.governorId, bind])
  );

  const enteringBind = existingByGovernorId.get(governorId);
  if (enteringBind && enteringBind.discordId !== discordId) {
    throw new BindServiceError(
      "That governor is already bound by another account."
    );
  }

  const enteringAlreadyVisible =
    enteringBind?.discordId === discordId && !hasPendingDelete(enteringBind)
      ? enteringBind.isVisible
      : false;
  if (!enteringAlreadyVisible && activeVisibleCount >= 10) {
    throw new BindServiceError("You can only have up to 10 visible binds.");
  }

  const shouldSetEnteredAsDefault = activeDefault == null;
  if (shouldSetEnteredAsDefault) {
    await bindsCollection.updateMany(activeFilter(discordId), {
      $set: {
        isDefault: false,
        updatedAt: now,
      },
    });
  }

  const summary: BindMutationSummary = {
    created: 0,
    updated: 0,
    skipped: 0,
  };

  for (const snapshot of candidateSnapshots) {
    const isEnteredGovernor = snapshot.governorId === governorId;
    const existing = existingByGovernorId.get(snapshot.governorId);

    const nextValues = {
      discordId,
      governorId: snapshot.governorId,
      kingdom: snapshot.kingdom,
      appUid: snapshot.appUid,
      name: snapshot.name,
      avatarUrl: snapshot.avatarUrl,
      frameUrl: snapshot.frameUrl,
      type: (isEnteredGovernor ? "main" : "farm") as UserBindType,
      isDefault: shouldSetEnteredAsDefault && isEnteredGovernor,
      isVisible: isEnteredGovernor,
      updatedAt: now,
    };

    if (!existing) {
      await bindsCollection.insertOne({
        ...nextValues,
        createdAt: now,
      });
      summary.created += 1;
      continue;
    }

    if (existing.discordId !== discordId) {
      summary.skipped += 1;
      continue;
    }

    const update: {
      $set: typeof nextValues;
      $unset?: Record<string, "">;
    } = {
      $set: nextValues,
    };

    if (hasPendingDelete(existing)) {
      update.$unset = { pendingDeleteAt: "" };
    }

    await bindsCollection.updateOne({ _id: existing._id }, update);
    summary.updated += 1;
  }

  return summary;
}

export async function makeBindDefault(
  db: Db,
  discordId: string,
  bindId: string
): Promise<void> {
  const bindsCollection = db.collection<UserBindFields>("user_binds");
  const now = new Date();
  const objectId = toObjectId(bindId);

  const bind = await bindsCollection.findOne({ _id: objectId, discordId });
  if (!bind) {
    throw new BindServiceError("Bind not found.");
  }

  if (hasPendingDelete(bind)) {
    throw new BindServiceError(
      "You cannot set a pending deletion bind as default."
    );
  }

  await bindsCollection.updateMany(activeFilter(discordId), {
    $set: {
      isDefault: false,
      updatedAt: now,
    },
  });

  await bindsCollection.updateOne(
    {
      _id: objectId,
    },
    {
      $set: {
        isDefault: true,
        isVisible: true,
        updatedAt: now,
      },
    }
  );
}

export async function setBindVisibility(
  db: Db,
  discordId: string,
  bindId: string,
  visible: boolean
): Promise<void> {
  const bindsCollection = db.collection<UserBindFields>("user_binds");
  const now = new Date();
  const objectId = toObjectId(bindId);

  const bind = await bindsCollection.findOne({ _id: objectId, discordId });
  if (!bind) {
    throw new BindServiceError("Bind not found.");
  }

  if (hasPendingDelete(bind)) {
    throw new BindServiceError(
      "You cannot change visibility while unlink is pending."
    );
  }

  if (bind.isDefault) {
    throw new BindServiceError("The default bind must always stay visible.");
  }

  if (visible) {
    const visibleCount = await bindsCollection.countDocuments({
      ...activeFilter(discordId),
      _id: { $ne: objectId },
      isVisible: true,
    });

    if (visibleCount >= 10) {
      throw new BindServiceError("You can only have up to 10 visible binds.");
    }
  }

  await bindsCollection.updateOne(
    { _id: objectId },
    {
      $set: {
        isVisible: visible,
        updatedAt: now,
      },
    }
  );
}

export async function unlinkBind(
  db: Db,
  discordId: string,
  bindId: string
): Promise<void> {
  const bindsCollection = db.collection<UserBindFields>("user_binds");
  const now = new Date();
  const pendingDeleteAt = new Date(now.getTime() + 24 * 60 * 60 * 1000);
  const objectId = toObjectId(bindId);

  const bind = await bindsCollection.findOne({ _id: objectId, discordId });
  if (!bind) {
    throw new BindServiceError("Bind not found.");
  }

  if (hasPendingDelete(bind)) {
    return;
  }

  if (bind.appUid != null && bind.type !== "main" && !bind.isDefault) {
    throw new BindServiceError(
      "Only the default bind can unlink an app UID group."
    );
  }

  const groupFilter =
    bind.appUid == null
      ? { _id: objectId }
      : {
          discordId,
          appUid: bind.appUid,
          pendingDeleteAt: { $exists: false },
        };

  const groupBinds = await bindsCollection.find(groupFilter).toArray();
  if (groupBinds.length === 0) {
    return;
  }

  const groupBindIds = groupBinds.map((item) => item._id);
  const defaultBindInGroup = groupBinds.find((item) => item.isDefault) ?? null;
  const groupContainsDefault = defaultBindInGroup != null;

  let replacement: UserBindDocument | null = null;
  if (groupContainsDefault) {
    replacement = await bindsCollection
      .find({
        ...activeFilter(discordId),
        _id: { $nin: groupBindIds },
      })
      .sort({ isVisible: -1, updatedAt: -1, createdAt: -1 })
      .limit(1)
      .next();
  }

  await bindsCollection.updateMany(
    {
      _id: { $in: groupBindIds },
    },
    {
      $set: {
        pendingDeleteAt,
        isDefault: false,
        isVisible: false,
        updatedAt: now,
      },
    }
  );

  if (!groupContainsDefault) {
    return;
  }

  if (!replacement) {
    if (defaultBindInGroup) {
      await bindsCollection.updateOne(
        { _id: defaultBindInGroup._id },
        {
          $set: {
            isDefault: true,
            isVisible: true,
            updatedAt: now,
          },
        }
      );
    }
    return;
  }

  await bindsCollection.updateMany(activeFilter(discordId), {
    $set: {
      isDefault: false,
      updatedAt: now,
    },
  });

  await bindsCollection.updateOne(
    {
      _id: replacement._id,
    },
    {
      $set: {
        isDefault: true,
        isVisible: true,
        updatedAt: now,
      },
    }
  );
}

export async function cancelBindUnlink(
  db: Db,
  discordId: string,
  bindId: string
): Promise<void> {
  const bindsCollection = db.collection<UserBindFields>("user_binds");
  const now = new Date();
  const objectId = toObjectId(bindId);

  const bind = await bindsCollection.findOne({ _id: objectId, discordId });
  if (!bind) {
    throw new BindServiceError("Bind not found.");
  }

  const restoreFilter =
    bind.appUid == null
      ? { _id: objectId, discordId }
      : { discordId, appUid: bind.appUid };

  await bindsCollection.updateMany(restoreFilter, {
    $unset: {
      pendingDeleteAt: "",
    },
    $set: {
      updatedAt: now,
    },
  });

  const activeDefaultCount = await bindsCollection.countDocuments({
    ...activeFilter(discordId),
    isDefault: true,
  });

  if (activeDefaultCount > 0) {
    return;
  }

  await bindsCollection.updateMany(activeFilter(discordId), {
    $set: {
      isDefault: false,
      updatedAt: now,
    },
  });

  await bindsCollection.updateOne(
    {
      _id: objectId,
      discordId,
      pendingDeleteAt: { $exists: false },
    },
    {
      $set: {
        isDefault: true,
        isVisible: true,
        updatedAt: now,
      },
    }
  );
}

async function upsertDiscoveredAppUidGroup(
  db: Db,
  discordId: string,
  appUid: number,
  now: Date
): Promise<BindMutationSummary> {
  const bindsCollection = db.collection<UserBindFields>("user_binds");
  const snapshots = await hydrateGovernorProfiles(
    db,
    await lookupAssociatedSnapshotsByAppUid(db, appUid)
  );

  if (snapshots.length === 0) {
    return {
      created: 0,
      updated: 0,
      skipped: 0,
    };
  }

  const governorIds = snapshots.map((snapshot) => snapshot.governorId);
  const existing = await bindsCollection
    .find({ governorId: { $in: governorIds } })
    .toArray();
  const existingByGovernorId = new Map(
    existing.map((bind) => [bind.governorId, bind])
  );

  const summary: BindMutationSummary = {
    created: 0,
    updated: 0,
    skipped: 0,
  };

  for (const snapshot of snapshots) {
    const existingBind = existingByGovernorId.get(snapshot.governorId);
    if (!existingBind) {
      await bindsCollection.insertOne({
        discordId,
        governorId: snapshot.governorId,
        kingdom: snapshot.kingdom,
        appUid,
        name: snapshot.name,
        avatarUrl: snapshot.avatarUrl,
        frameUrl: snapshot.frameUrl,
        type: "farm",
        isDefault: false,
        isVisible: false,
        createdAt: now,
        updatedAt: now,
      });
      summary.created += 1;
      continue;
    }

    if (existingBind.discordId !== discordId) {
      summary.skipped += 1;
      continue;
    }

    const update: {
      $set: {
        kingdom: number | null;
        appUid: number;
        name: string | null;
        avatarUrl: string | null;
        frameUrl: string | null;
        updatedAt: Date;
      };
      $unset?: Record<string, "">;
    } = {
      $set: {
        kingdom: snapshot.kingdom,
        appUid,
        name: snapshot.name,
        avatarUrl: snapshot.avatarUrl,
        frameUrl: snapshot.frameUrl,
        updatedAt: now,
      },
    };

    if (hasPendingDelete(existingBind)) {
      update.$unset = { pendingDeleteAt: "" };
    }

    await bindsCollection.updateOne(
      {
        _id: existingBind._id,
      },
      update
    );
    summary.updated += 1;
  }

  return summary;
}

export async function refreshBinds(db: Db): Promise<RefreshBindsSummary> {
  const bindsCollection = db.collection<UserBindFields>("user_binds");
  const summary: RefreshBindsSummary = {
    processed: 0,
    updated: 0,
    inserted: 0,
    skipped: 0,
    errors: 0,
  };

  const activeBinds = await bindsCollection
    .find({ pendingDeleteAt: { $exists: false } })
    .toArray();

  const appUidGroupsByUser = new Map<string, Set<number>>();
  const noAppUidBinds: UserBindDocument[] = [];

  for (const bind of activeBinds) {
    if (bind.appUid == null) {
      noAppUidBinds.push(bind);
      continue;
    }

    const appUids = appUidGroupsByUser.get(bind.discordId) ?? new Set<number>();
    appUids.add(bind.appUid);
    appUidGroupsByUser.set(bind.discordId, appUids);
  }

  for (const [discordId, appUids] of appUidGroupsByUser) {
    for (const appUid of appUids) {
      summary.processed += 1;
      const now = new Date();

      try {
        const appUidSummary = await upsertDiscoveredAppUidGroup(
          db,
          discordId,
          appUid,
          now
        );
        summary.inserted += appUidSummary.created;
        summary.updated += appUidSummary.updated;
        summary.skipped += appUidSummary.skipped;
      } catch {
        summary.errors += 1;
      }
    }
  }

  for (const bind of noAppUidBinds) {
    summary.processed += 1;
    const now = new Date();

    try {
      const latestSnapshot = await lookupLatestGovernorSnapshot(
        db,
        bind.governorId
      );

      if (!latestSnapshot) {
        const fallbackProfile = await lookupKingdomGovernorProfile(
          db,
          bind.governorId
        );
        await bindsCollection.updateOne(
          { _id: bind._id },
          {
            $set: {
              name: fallbackProfile.name,
              kingdom: fallbackProfile.kingdom,
              updatedAt: now,
            },
          }
        );
        summary.updated += 1;
        continue;
      }

      const [hydratedSnapshot] = await hydrateGovernorProfiles(db, [
        latestSnapshot,
      ]);
      await bindsCollection.updateOne(
        { _id: bind._id },
        {
          $set: {
            kingdom: hydratedSnapshot.kingdom,
            appUid: hydratedSnapshot.appUid,
            name: hydratedSnapshot.name,
            avatarUrl: hydratedSnapshot.avatarUrl,
            frameUrl: hydratedSnapshot.frameUrl,
            updatedAt: now,
          },
        }
      );
      summary.updated += 1;

      if (hydratedSnapshot.appUid != null) {
        const appUidSummary = await upsertDiscoveredAppUidGroup(
          db,
          bind.discordId,
          hydratedSnapshot.appUid,
          now
        );
        summary.inserted += appUidSummary.created;
        summary.updated += appUidSummary.updated;
        summary.skipped += appUidSummary.skipped;
      }
    } catch {
      summary.errors += 1;
    }
  }

  return summary;
}
