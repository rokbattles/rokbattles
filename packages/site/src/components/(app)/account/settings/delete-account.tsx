"use client";

import { useRef, useState } from "react";
import { Alert, AlertActions, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Link } from "@/components/ui/link";

export function DeleteAccount() {
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submitting = useRef(false);

  function openConfirmation() {
    setError(null);
    setOpen(true);
  }

  async function deleteAccount() {
    if (submitting.current) {
      return;
    }

    submitting.current = true;
    setPending(true);
    setError(null);

    try {
      const response = await fetch("/proxy/v1/auth/account", {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ confirmed: true }),
        cache: "no-store",
      });

      if (response.ok) {
        window.location.replace("/");
        return;
      }

      if (response.status === 401) {
        setError("Your session ended before deletion could be confirmed. Sign in and try again.");
        submitting.current = false;
        setPending(false);
        return;
      }

      throw new Error("Account deletion failed");
    } catch {
      setError("We couldn't finish deleting your account. Please try again.");
      submitting.current = false;
      setPending(false);
    }
  }

  return (
    <>
      <Button color="red" onClick={openConfirmation}>
        Delete account
      </Button>
      <Alert open={open} onClose={() => !submitting.current && setOpen(false)}>
        <AlertTitle>Delete your account?</AlertTitle>
        <AlertDescription>
          This will permanently delete your ROK Battles account and governor binds, and log you out
          of all devices. This action cannot be undone.
        </AlertDescription>
        {error ? (
          <div role="alert" className="mt-4 space-y-2 text-sm text-red-400">
            <p>{error}</p>
            <Link href="/proxy/v1/auth/discord/login" prefetch={false} className="underline">
              Sign in again
            </Link>
          </div>
        ) : null}
        <AlertActions>
          <Button plain data-autofocus disabled={pending} onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button color="red" disabled={pending} onClick={deleteAccount}>
            {pending ? "Deleting…" : "Delete account"}
          </Button>
        </AlertActions>
      </Alert>
    </>
  );
}
