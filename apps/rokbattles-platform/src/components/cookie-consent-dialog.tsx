"use client";

import { useTranslations } from "next-intl";
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
  const t = useTranslations("cookieConsent");

  return (
    <Dialog open={open} onClose={() => onClose()} size="lg">
      <DialogTitle>{t("title")}</DialogTitle>
      <DialogDescription>
        {t.rich("message", {
          link: (chunks) => (
            <TextLink
              href="https://rokbattles.com/legal/cookie-policy"
              target="_blank"
              rel="noopener noreferrer"
            >
              {chunks}
            </TextLink>
          ),
        })}
      </DialogDescription>
      <DialogActions>
        <Button
          onClick={() => {
            updateCookieConsent(false);
            onClose();
          }}
        >
          {t("reject")}
        </Button>
        <Button
          color="blue"
          onClick={() => {
            updateCookieConsent(true);
            onClose();
          }}
        >
          {t("accept")}
        </Button>
      </DialogActions>
    </Dialog>
  );
}
