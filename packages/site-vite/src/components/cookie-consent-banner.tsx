import { use } from "react";
import {
  CookieConsentContext,
  defaultCookiePreferences,
} from "../providers/cookie-consent-context";
import { Button } from "./ui/marketing/button";
import { Link } from "./ui/marketing/link";

export function CookieConsentBanner() {
  const { consent, isOpen, open, update } = use(CookieConsentContext);

  if (consent) return null;

  return (
    <section
      aria-label="Cookie notice"
      hidden={isOpen}
      className="fixed right-2 bottom-2 z-50 max-h-[calc(100dvh-1rem)] w-[calc(100%-1rem)] overflow-y-auto rounded-sm border border-white/10 bg-zinc-900 p-4 text-white shadow-xl sm:right-4 sm:bottom-4 sm:w-96"
    >
      <h2 className="text-sm/6 font-semibold">Your cookie preferences</h2>
      <p className="mt-2 text-sm/6 text-zinc-400">
        We use necessary cookies to keep ROK Battles working. You can choose your optional cookie
        preferences.
      </p>
      <div className="mt-4 grid grid-cols-2 gap-3">
        <Button size="compact" onClick={() => update(defaultCookiePreferences)}>
          Reject optional
        </Button>
        <Button
          size="compact"
          variant="primary"
          onClick={() => update({ functional: true, analytics: true, marketing: true })}
        >
          Accept all
        </Button>
      </div>
      <div className="mt-2 flex flex-wrap items-center justify-between gap-x-4">
        <Link href="/legal/cookie-policy" className="underline">
          Cookie policy
        </Link>
        <button
          type="button"
          aria-haspopup="dialog"
          onClick={open}
          className="min-h-11 cursor-pointer text-base/7 text-zinc-400 underline hover:text-white focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400 sm:text-sm/7"
        >
          Manage preferences
        </button>
      </div>
    </section>
  );
}
