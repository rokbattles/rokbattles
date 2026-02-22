import "server-only";

import { normalizeTimestampMillis } from "@/lib/datetime";
import type {
  ArkBattleInfoMailDocument,
  ArkBattleResultsMailDocument,
  ArkIndividualResultsMailDocument,
} from "@/lib/types/ark";

type MailDocumentWithMetadata = {
  metadata?: {
    mail_id?: unknown;
    mail_time?: unknown;
  } | null;
};

type MatchCandidate<T> = {
  doc: T;
  mailId: string | null;
  mailTimeMillis: number;
};

type MatchedSecondary<T> = {
  candidate: MatchCandidate<T>;
  deltaMillis: number;
};

export type MatchedArkMailSet = {
  battleResults: ArkBattleResultsMailDocument;
  battleResultsMailId: string | null;
  battleResultsTimeMillis: number;
  battleInfo: ArkBattleInfoMailDocument | null;
  battleInfoMailId: string | null;
  battleInfoDeltaMillis: number | null;
  individualResults: ArkIndividualResultsMailDocument | null;
  individualResultsMailId: string | null;
  individualResultsDeltaMillis: number | null;
};

function toMailId(value: unknown): string | null {
  if (typeof value === "string") {
    const trimmed = value.trim();
    return trimmed === "" ? null : trimmed;
  }

  if (typeof value === "number" && Number.isFinite(value)) {
    return value.toString();
  }

  if (typeof value === "bigint") {
    return value.toString();
  }

  return null;
}

function toCandidate<T extends MailDocumentWithMetadata>(doc: T): MatchCandidate<T> | null {
  const mailTimeMillis = normalizeTimestampMillis(doc.metadata?.mail_time);
  if (mailTimeMillis == null) {
    return null;
  }

  return {
    doc,
    mailId: toMailId(doc.metadata?.mail_id),
    mailTimeMillis,
  };
}

function isBetterCandidate<T>(
  candidate: MatchCandidate<T>,
  deltaMillis: number,
  bestCandidate: MatchCandidate<T> | null,
  bestDeltaMillis: number
): boolean {
  if (!bestCandidate) {
    return true;
  }

  if (deltaMillis !== bestDeltaMillis) {
    return deltaMillis < bestDeltaMillis;
  }

  if (candidate.mailTimeMillis !== bestCandidate.mailTimeMillis) {
    return candidate.mailTimeMillis > bestCandidate.mailTimeMillis;
  }

  const candidateId = candidate.mailId ?? "";
  const bestId = bestCandidate.mailId ?? "";
  return candidateId < bestId;
}

function consumeBestCandidate<T>(
  pool: Array<MatchCandidate<T> | null>,
  primaryTimeMillis: number,
  maxDeltaMillis: number
): MatchedSecondary<T> | null {
  let bestIndex = -1;
  let bestCandidate: MatchCandidate<T> | null = null;
  let bestDeltaMillis = Number.MAX_SAFE_INTEGER;

  for (let index = 0; index < pool.length; index += 1) {
    const candidate = pool[index];
    if (!candidate) {
      continue;
    }

    const deltaMillis = Math.abs(candidate.mailTimeMillis - primaryTimeMillis);
    if (deltaMillis > maxDeltaMillis) {
      continue;
    }

    if (isBetterCandidate(candidate, deltaMillis, bestCandidate, bestDeltaMillis)) {
      bestCandidate = candidate;
      bestDeltaMillis = deltaMillis;
      bestIndex = index;
    }
  }

  if (bestIndex < 0 || !bestCandidate) {
    return null;
  }

  pool[bestIndex] = null;

  return {
    candidate: bestCandidate,
    deltaMillis: bestDeltaMillis,
  };
}

export function matchArkMails(options: {
  battleResults: ArkBattleResultsMailDocument[];
  battleInfo: ArkBattleInfoMailDocument[];
  individualResults: ArkIndividualResultsMailDocument[];
  maxDeltaMillis?: number;
}): MatchedArkMailSet[] {
  const { battleResults, battleInfo, individualResults, maxDeltaMillis = 60_000 } = options;
  const safeMaxDeltaMillis =
    Number.isFinite(maxDeltaMillis) && maxDeltaMillis > 0 ? Math.floor(maxDeltaMillis) : 60_000;

  const battleInfoPool = battleInfo.map((doc) => toCandidate(doc));
  const individualResultsPool = individualResults.map((doc) => toCandidate(doc));
  const matches: MatchedArkMailSet[] = [];

  for (const battleResultsDoc of battleResults) {
    const primaryCandidate = toCandidate(battleResultsDoc);
    if (!primaryCandidate) {
      continue;
    }

    const matchedBattleInfo = consumeBestCandidate(
      battleInfoPool,
      primaryCandidate.mailTimeMillis,
      safeMaxDeltaMillis
    );
    const matchedIndividualResults = consumeBestCandidate(
      individualResultsPool,
      primaryCandidate.mailTimeMillis,
      safeMaxDeltaMillis
    );

    matches.push({
      battleResults: battleResultsDoc,
      battleResultsMailId: primaryCandidate.mailId,
      battleResultsTimeMillis: primaryCandidate.mailTimeMillis,
      battleInfo: matchedBattleInfo?.candidate.doc ?? null,
      battleInfoMailId: matchedBattleInfo?.candidate.mailId ?? null,
      battleInfoDeltaMillis: matchedBattleInfo?.deltaMillis ?? null,
      individualResults: matchedIndividualResults?.candidate.doc ?? null,
      individualResultsMailId: matchedIndividualResults?.candidate.mailId ?? null,
      individualResultsDeltaMillis: matchedIndividualResults?.deltaMillis ?? null,
    });
  }

  return matches;
}
