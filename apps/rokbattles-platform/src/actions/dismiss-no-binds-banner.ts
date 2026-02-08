"use server";

import { revalidatePath } from "next/cache";
import { cookies } from "next/headers";

export async function dismissNoBindsBannerAction(): Promise<void> {
  const cookieStore = await cookies();
  cookieStore.set("hideNoBindsBanner", "1", {
    httpOnly: true,
    path: "/",
    sameSite: "lax",
    secure: process.env.NODE_ENV === "production",
  });

  revalidatePath("/account/settings");
}
