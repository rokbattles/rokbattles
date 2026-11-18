"use client";

import { useRouter } from "next/navigation";
import { type FormEvent, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox, CheckboxField } from "@/components/ui/checkbox";
import { Dialog, DialogActions, DialogBody, DialogTitle } from "@/components/ui/dialog";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { TextLink } from "@/components/ui/text";
import { createDeveloperApp } from "@/lib/(app)/developer-apps";

export function CreateAppDialog() {
  const router = useRouter();
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");

  function setDialogOpen(next: boolean) {
    if (!pending) {
      setOpen(next);
      setError("");
    }
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    setPending(true);
    setError("");

    try {
      const app = await createDeveloperApp({
        name: String(data.get("name") ?? ""),
        acceptTerms: data.has("acceptTerms"),
      });
      setOpen(false);
      router.push(`/developer/apps/${app.id}`);
      router.refresh();
    } catch (error) {
      setError(error instanceof Error ? error.message : "The app could not be created.");
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <Button onClick={() => setDialogOpen(true)}>Create app</Button>
      {open ? (
        <Dialog open onClose={() => setDialogOpen(false)}>
          <DialogTitle>Create an app</DialogTitle>
          <form onSubmit={submit}>
            <DialogBody className="space-y-6">
              <Field disabled={pending}>
                <Label>App name</Label>
                <Input name="name" required />
              </Field>
              <CheckboxField disabled={pending}>
                <Checkbox name="acceptTerms" required />
                <Label>
                  I agree to the{" "}
                  <TextLink href="/legal/terms-of-service" target="_blank" rel="noreferrer">
                    ROK Battles Terms of Service
                  </TextLink>
                  .
                </Label>
              </CheckboxField>
              {error ? (
                <p role="alert" className="text-sm text-red-400">
                  {error}
                </p>
              ) : null}
            </DialogBody>
            <DialogActions>
              <Button type="button" plain disabled={pending} onClick={() => setDialogOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={pending}>
                {pending ? "Creating…" : "Create app"}
              </Button>
            </DialogActions>
          </form>
        </Dialog>
      ) : null}
    </>
  );
}
