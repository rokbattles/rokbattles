"use client";

import { Eye, EyeOff } from "lucide-react";
import { useState } from "react";

export function AccountEmail({ request, maskedEmail }: { request: string; maskedEmail: string }) {
  const [email, setEmail] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [failed, setFailed] = useState(false);
  const label = email === null ? "Reveal email address" : "Hide email address";
  const Icon = email === null ? Eye : EyeOff;

  async function toggleEmail() {
    if (email !== null) {
      setEmail(null);
      return;
    }

    setPending(true);
    setFailed(false);

    try {
      const query = new URLSearchParams({ request });
      const response = await fetch(`/proxy/v1/auth/authorize/consent/email?${query}`, {
        cache: "no-store",
      });

      if (!response.ok) {
        setFailed(true);
        return;
      }

      const account: { email: string } = await response.json();
      setEmail(account.email);
    } catch {
      setFailed(true);
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <div className="flex items-center gap-1">
        <p className="min-w-0 wrap-anywhere" aria-live="polite">
          {email ?? maskedEmail}
        </p>
        <button
          type="button"
          aria-label={label}
          title={label}
          disabled={pending}
          onClick={toggleEmail}
          className="grid size-6 shrink-0 cursor-pointer place-items-center rounded hover:bg-white/10 hover:text-zinc-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-orange-300 disabled:cursor-wait"
        >
          <Icon aria-hidden="true" className="size-4" />
        </button>
      </div>
      {failed ? (
        <p className="text-xs text-orange-200" role="status">
          Couldn’t show your email. Try again.
        </p>
      ) : null}
    </>
  );
}
