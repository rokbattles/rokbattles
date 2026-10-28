"use client";

import { Button } from "@/components/ui/button";
import { Dialog, DialogActions, DialogDescription, DialogTitle } from "@/components/ui/dialog";
import { TextLink } from "@/components/ui/text";
import { useCookieConsent } from "@/providers/cookie-consent-context";

type CookieConsentDialogProps = {
  open: boolean;
  onClose: () => void;
};

export function CookieConsentDialog({ open, onClose }: CookieConsentDialogProps) {
  const { updateCookieConsent } = useCookieConsent();

  return (
    <Dialog open={open} onClose={() => onClose()} size="lg">
      <DialogTitle>Cookie settings</DialogTitle>
      <DialogDescription>
        We use only essential cookies for authentication, security, and site functionality. If we
        add optional cookies in the future, you'll be able to manage them here. Read our{" "}
        <TextLink href="/legal/cookie-policy">cookie policy</TextLink> for more information.
      </DialogDescription>
      <DialogActions>
        <Button
          onClick={() => {
            updateCookieConsent(false);
            onClose();
          }}
        >
          Reject
        </Button>
        <Button
          color="blue"
          onClick={() => {
            updateCookieConsent(true);
            onClose();
          }}
        >
          Accept
        </Button>
      </DialogActions>
    </Dialog>
  );
}
