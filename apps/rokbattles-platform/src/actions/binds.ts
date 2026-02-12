"use server";

import { revalidatePath } from "next/cache";
import { fetchCurrentUser } from "@/data/fetch-current-user";
import {
  BindServiceError,
  cancelBindUnlink,
  createBindsFromGovernor,
  makeBindDefault,
  parseGovernorIdInput,
  setBindVisibility,
  unlinkBind,
} from "@/lib/bind";
import clientPromise from "@/lib/mongo";

interface ActionResult {
  ok: boolean;
  error?: string;
}

async function withAuthenticatedUser<T>(
  handler: (args: { discordId: string }) => Promise<T>
): Promise<T> {
  const currentUser = await fetchCurrentUser();
  if (!currentUser) {
    throw new BindServiceError("You need to sign in first.");
  }

  return handler({ discordId: currentUser.discordId });
}

function toActionError(error: unknown): ActionResult {
  if (error instanceof BindServiceError) {
    return {
      ok: false,
      error: error.message,
    };
  }

  return {
    ok: false,
    error: "Something went wrong. Please try again.",
  };
}

export async function createBindAction(input: {
  governorId: string;
}): Promise<ActionResult> {
  try {
    const governorId = parseGovernorIdInput(input.governorId);

    await withAuthenticatedUser(async ({ discordId }) => {
      const client = await clientPromise;
      if (!client) {
        throw new BindServiceError("MongoDB client unavailable.");
      }

      const db = client.db();
      await createBindsFromGovernor(db, discordId, governorId);
    });

    revalidatePath("/account/settings");
    return { ok: true };
  } catch (error) {
    return toActionError(error);
  }
}

export async function makeDefaultBindAction(input: {
  bindId: string;
}): Promise<ActionResult> {
  try {
    await withAuthenticatedUser(async ({ discordId }) => {
      const client = await clientPromise;
      if (!client) {
        throw new BindServiceError("MongoDB client unavailable.");
      }

      await makeBindDefault(client.db(), discordId, input.bindId);
    });

    revalidatePath("/account/settings");
    return { ok: true };
  } catch (error) {
    return toActionError(error);
  }
}

export async function setBindVisibilityAction(input: {
  bindId: string;
  visible: boolean;
}): Promise<ActionResult> {
  try {
    await withAuthenticatedUser(async ({ discordId }) => {
      const client = await clientPromise;
      if (!client) {
        throw new BindServiceError("MongoDB client unavailable.");
      }

      await setBindVisibility(
        client.db(),
        discordId,
        input.bindId,
        input.visible
      );
    });

    revalidatePath("/account/settings");
    return { ok: true };
  } catch (error) {
    return toActionError(error);
  }
}

export async function unlinkBindAction(input: {
  bindId: string;
}): Promise<ActionResult> {
  try {
    await withAuthenticatedUser(async ({ discordId }) => {
      const client = await clientPromise;
      if (!client) {
        throw new BindServiceError("MongoDB client unavailable.");
      }

      await unlinkBind(client.db(), discordId, input.bindId);
    });

    revalidatePath("/account/settings");
    return { ok: true };
  } catch (error) {
    return toActionError(error);
  }
}

export async function cancelUnlinkBindAction(input: {
  bindId: string;
}): Promise<ActionResult> {
  try {
    await withAuthenticatedUser(async ({ discordId }) => {
      const client = await clientPromise;
      if (!client) {
        throw new BindServiceError("MongoDB client unavailable.");
      }

      await cancelBindUnlink(client.db(), discordId, input.bindId);
    });

    revalidatePath("/account/settings");
    return { ok: true };
  } catch (error) {
    return toActionError(error);
  }
}
