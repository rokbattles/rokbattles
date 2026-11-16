import type { JSX } from "react";
import { Alert, AlertActions, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import type { OverlayUrlAction } from "@/lib/(app)/types/overlay";

type OverlayUrlConfirmationProps = {
  action: OverlayUrlAction;
  open: boolean;
  busy: boolean;
  onClose: () => void;
  onConfirm: () => void;
};

export function OverlayUrlConfirmation({
  action,
  open,
  busy,
  onClose,
  onConfirm,
}: OverlayUrlConfirmationProps): JSX.Element {
  const rotating = action === "rotate";
  const title = rotating ? "Rotate overlay URL?" : "Disable overlay?";
  const description = rotating
    ? "Your current Live and Test URLs will stop working. Replace them in your streaming software with the new URLs."
    : "Your Live and Test URLs will stop working. Your settings will be kept, and you can create a new URL later.";
  const label = rotating ? "Rotate URL" : "Disable";
  const color: "zinc" | "red" = rotating ? "zinc" : "red";

  return (
    <Alert open={open} onClose={() => !busy && onClose()}>
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>{description}</AlertDescription>
      <AlertActions>
        <Button plain data-autofocus disabled={busy} onClick={onClose}>
          Cancel
        </Button>
        <Button color={color} disabled={busy} onClick={onConfirm}>
          {busy ? "Updating…" : label}
        </Button>
      </AlertActions>
    </Alert>
  );
}
