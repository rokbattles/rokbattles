import { getExtracted } from "next-intl/server";
import { dismissNoBindsBannerAction } from "@/actions/dismiss-no-binds-banner";
import { Button } from "@/components/ui/button";

export async function NoBindsBanner() {
  const t = await getExtracted();

  return (
    <div className="rounded-xl border border-zinc-950/10 bg-zinc-50 px-4 py-3 dark:border-white/10 dark:bg-white/5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-zinc-900 dark:text-zinc-100">
          {t("Hey! It looks like you haven't bound an account yet.")}
        </p>
        <div className="flex shrink-0 items-center justify-end gap-2">
          <Button href="/account/settings#bind-form">
            {t("Bind an account")}
          </Button>
          <form action={dismissNoBindsBannerAction}>
            <Button outline type="submit">
              {t("Dismiss")}
            </Button>
          </form>
        </div>
      </div>
    </div>
  );
}
