"use client";

import { useRouter } from "next/navigation";
import { useState, useTransition } from "react";
import { Button } from "@/components/ui/button";
import { Text } from "@/components/ui/text";

export function DeauthorizeApp({ id, name }: { id: string; name: string }) {
  const router = useRouter();
  const [pending, startTransition] = useTransition();
  const [error, setError] = useState(false);

  function deauthorize() {
    setError(false);

    startTransition(async () => {
      try {
        const response = await fetch(`/proxy/v1/auth/authorized-apps/${encodeURIComponent(id)}`, {
          method: "DELETE",
        });
        if (!response.ok) {
          setError(true);
          return;
        }
        router.refresh();
      } catch {
        setError(true);
      }
    });
  }

  return (
    <div className="flex items-center justify-end gap-3">
      {error ? <Text role="alert">Could not deauthorize the app. Try again.</Text> : null}
      <Button
        aria-label={`Deauthorize ${name}`}
        disabled={pending}
        onClick={deauthorize}
        outline
        type="button"
      >
        {pending ? "Deauthorizing…" : "Deauthorize"}
      </Button>
    </div>
  );
}
