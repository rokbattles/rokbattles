"use client";

import { useRouter } from "next/navigation";
import { type FormEvent, useRef, useState } from "react";
import {
  ProgramApplicationRequestError,
  submitProgramApplication,
} from "@/lib/(app)/program-applications";
import type { ProgramApplication, ProgramDefinition } from "@/lib/(app)/types/program-application";

type SubmissionError = {
  message: string;
  requiresSignIn: boolean;
};

type ProgramApplicationOptions = {
  program: ProgramDefinition;
  onClose: () => void;
  onSubmitted: (application: ProgramApplication) => void;
  onConflict: () => Promise<void>;
};

function submissionError(error: unknown, program: ProgramDefinition): SubmissionError {
  const status = error instanceof ProgramApplicationRequestError ? error.status : null;
  let message: string;

  switch (status) {
    case 400:
    case 422:
      message = program.validationError;
      break;
    case 401:
      message = "Your session has ended. Sign in again before submitting.";
      break;
    case 403:
      message = "Verification failed or is unavailable. Please retry verification.";
      break;
    case 409:
      message = "You already have an open application or active access for this program.";
      break;
    case 429:
      message = "Too many attempts. Please wait a minute before trying again.";
      break;
    default:
      message =
        "Could not confirm your submission. Please try again; duplicate applications are prevented.";
  }

  return { message, requiresSignIn: status === 401 };
}

export function useProgramApplication({
  program,
  onClose,
  onSubmitted,
  onConflict,
}: ProgramApplicationOptions) {
  const router = useRouter();
  const submitting = useRef(false);
  const [token, setToken] = useState("");
  const [verificationAttempt, setVerificationAttempt] = useState(0);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<SubmissionError | null>(null);

  function close() {
    if (!submitting.current) onClose();
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (submitting.current || !token) return;

    const answer = String(new FormData(event.currentTarget).get("answer") ?? "").trim();

    if (!answer) {
      setError({ message: "Please answer the question before submitting.", requiresSignIn: false });
      return;
    }

    submitting.current = true;
    setPending(true);
    setError(null);

    try {
      const application = await submitProgramApplication(program.id, answer, token);

      onSubmitted(application);
      router.refresh();
    } catch (error) {
      if (error instanceof ProgramApplicationRequestError && error.status === 409) {
        await onConflict();
        router.refresh();
      }

      setError(submissionError(error, program));
    } finally {
      // Tokens are single-use. Remount the widget before another attempt.
      setToken("");
      setVerificationAttempt((attempt) => attempt + 1);
      setPending(false);
      submitting.current = false;
    }
  }

  return { pending, error, token, setToken, verificationAttempt, submit, close };
}
