"use client";

import Script from "next/script";
import { Button } from "@/components/ui/button";
import { useTurnstile } from "@/hooks/(app)/use-turnstile";

type TurnstileProps = {
  action: string;
  onVerify: (token: string) => void;
  siteKey?: string;
};

export function Turnstile({
  action,
  onVerify,
  siteKey = process.env.NEXT_PUBLIC_TURNSTILE_SITE_KEY,
}: TurnstileProps) {
  const widget = useTurnstile({ siteKey, action, onVerify });

  if (!siteKey) {
    return <p role="alert">Verification is unavailable. Please try again later.</p>;
  }

  return (
    <div className="space-y-3">
      <Script
        src="https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit"
        strategy="afterInteractive"
        onReady={widget.onReady}
        onError={widget.onLoadError}
      />
      <div ref={widget.container} />
      {widget.error ? (
        <div className="space-y-2">
          <p role="alert" className="text-sm text-red-400">
            {widget.error}
          </p>
          {widget.ready ? (
            <Button outline type="button" onClick={widget.retry}>
              Retry verification
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
