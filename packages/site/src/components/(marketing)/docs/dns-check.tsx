"use client";

import { useState } from "react";
import { Button } from "@/components/(marketing)/ui/button";

const results = {
  idle: "",
  running: "Checking this browser’s DNS connection…",
  confirmed: "ROK Battles DNS confirmed for this browser.",
  unconfirmed: "DNS could not be confirmed. Review your setup below and try again.",
  unavailable: "The DNS check is temporarily unavailable. Try again in a moment.",
};

export function DnsCheck() {
  const [result, setResult] = useState<keyof typeof results>("idle");

  async function runCheck(): Promise<void> {
    setResult("running");
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), 12_000);

    try {
      const bytes = crypto.getRandomValues(new Uint8Array(16));
      const label = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");

      const postCheck = (operation: "start" | "result") =>
        fetch(`/api/dns-check/${operation}`, {
          method: "POST",
          body: label,
          cache: "no-store",
          credentials: "omit",
          headers: { "Content-Type": "application/octet-stream" },
          redirect: "error",
          referrerPolicy: "no-referrer",
          signal: controller.signal,
        });

      if ((await postCheck("start")).status !== 204) {
        setResult("unavailable");
        return;
      }

      // Probe names return NXDOMAIN. The result endpoint reports whether the resolver saw the lookup.
      void fetch(`https://${label}.dns-probe.rokbattles.com/`, {
        cache: "no-store",
        credentials: "omit",
        redirect: "error",
        referrerPolicy: "no-referrer",
        signal: controller.signal,
      }).catch(() => {});

      const { status } = await postCheck("result");

      if (status === 204) {
        setResult("confirmed");
      } else if (status === 404) {
        setResult("unconfirmed");
      } else {
        setResult("unavailable");
      }
    } catch {
      setResult("unavailable");
    } finally {
      window.clearTimeout(timeout);
      controller.abort();
    }
  }

  return (
    <section
      aria-label="Browser DNS check"
      className="rounded-sm border border-white/15 bg-white/5 p-5 sm:p-6"
    >
      <Button variant="primary" disabled={result === "running"} onClick={runCheck}>
        {result === "idle" || result === "running" ? "Run DNS check" : "Run again"}
      </Button>
      <p role="status" className="mt-3 min-h-7 text-white">
        {results[result]}
      </p>
    </section>
  );
}
