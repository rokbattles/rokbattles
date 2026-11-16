"use client";

import { Turnstile } from "@/components/(app)/turnstile";
import { Button } from "@/components/ui/button";
import {
  Drawer,
  DrawerActions,
  DrawerBody,
  DrawerDescription,
  DrawerTitle,
} from "@/components/ui/drawer";
import { Description, Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { useProgramApplication } from "@/hooks/(app)/use-program-application";
import { PROGRAM_APPLICATION_ACTION } from "@/lib/(app)/program-applications";
import type { ProgramApplication, ProgramDefinition } from "@/lib/(app)/types/program-application";

type ProgramApplicationDrawerProps = {
  program: ProgramDefinition;
  onClose: () => void;
  onSubmitted: (application: ProgramApplication) => void;
  onConflict: () => Promise<void>;
};

export function ProgramApplicationDrawer(props: ProgramApplicationDrawerProps) {
  const { program } = props;
  const form = useProgramApplication(props);

  return (
    <Drawer open side="right" size="xl" onClose={form.close}>
      <DrawerTitle>{program.title}</DrawerTitle>
      <DrawerDescription>{program.description}</DrawerDescription>
      <form onSubmit={form.submit} className="flex min-h-0 flex-1 flex-col">
        <DrawerBody className="space-y-6">
          <Field disabled={form.pending}>
            <Label>{program.question}</Label>
            {program.helpText ? <Description>{program.helpText}</Description> : null}
            {program.input === "url" ? (
              <Input
                name="answer"
                type="url"
                required
                maxLength={program.maxLength}
                placeholder={program.placeholder}
              />
            ) : (
              <Textarea name="answer" required maxLength={program.maxLength} rows={7} />
            )}
          </Field>
          <Turnstile
            key={form.verificationAttempt}
            action={PROGRAM_APPLICATION_ACTION}
            onVerify={form.setToken}
          />
          {form.error ? (
            <div role="alert" className="space-y-2 text-sm text-red-400">
              <p>{form.error.message}</p>
              {form.error.requiresSignIn ? (
                <Button href="/proxy/v1/auth/discord/login">Sign in again</Button>
              ) : null}
            </div>
          ) : null}
        </DrawerBody>
        <DrawerActions>
          <Button plain disabled={form.pending} onClick={form.close}>
            Cancel
          </Button>
          <Button type="submit" disabled={form.pending || !form.token}>
            {form.pending ? "Submitting…" : "Submit application"}
          </Button>
        </DrawerActions>
      </form>
    </Drawer>
  );
}
