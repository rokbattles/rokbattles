import { X } from "lucide-react";
import { use, useState } from "react";
import {
  CookieConsentContext,
  defaultCookiePreferences,
  type OptionalCookiePreferences,
} from "../providers/cookie-consent-context";
import { Drawer, DrawerBody, DrawerDescription, DrawerPanel, DrawerTitle } from "./ui/drawer";
import { Description, Label } from "./ui/fieldset";
import { Button } from "./ui/marketing/button";
import { Link } from "./ui/marketing/link";
import { Switch, SwitchField, SwitchGroup } from "./ui/switch";

const optionalCategories = [
  {
    key: "functional",
    label: "Functional",
    description: "Remember preferences and provide personalized features.",
  },
  {
    key: "analytics",
    label: "Analytics",
    description: "Help us understand how the site is used and improve it.",
  },
  {
    key: "marketing",
    label: "Marketing",
    description: "Personalize advertising and measure campaigns.",
  },
] as const;

function CookieConsentForm() {
  const { consent, close, update } = use(CookieConsentContext);
  const [preferences, setPreferences] = useState<OptionalCookiePreferences>(
    () => consent?.categories ?? defaultCookiePreferences
  );

  return (
    <>
      <div className="flex shrink-0 items-center justify-between gap-4">
        <DrawerTitle>Cookie settings</DrawerTitle>
        <Button variant="plain" size="icon" aria-label="Close cookie settings" onClick={close}>
          <X aria-hidden="true" />
        </Button>
      </div>
      <DrawerBody className="mt-3">
        <DrawerDescription className="mt-0">
          We use necessary cookies to keep ROK Battles working. You can choose your optional cookie
          preferences.
        </DrawerDescription>
        <Link href="/legal/cookie-policy" onClick={close} className="mt-2 underline">
          Read our cookie policy
        </Link>
        <SwitchGroup className="mt-6 divide-y divide-white/10">
          <SwitchField className="pb-6">
            <Label>Strictly necessary</Label>
            <Description>
              Required for authentication, security, and core site features. Always on.
            </Description>
            <Switch checked disabled color="orange" />
          </SwitchField>
          {optionalCategories.map(({ key, label, description }) => (
            <SwitchField key={key} className="pb-6">
              <Label>{label}</Label>
              <Description>{description}</Description>
              <Switch
                color="orange"
                checked={preferences[key] === true}
                onCheckedChange={(checked) =>
                  setPreferences((current) => ({ ...current, [key]: checked }))
                }
              />
            </SwitchField>
          ))}
        </SwitchGroup>
      </DrawerBody>
      <div className="mt-6 grid shrink-0 gap-3 border-t border-white/10 pt-6">
        <div className="grid grid-cols-2 gap-3">
          <Button onClick={() => update(defaultCookiePreferences)}>Reject optional</Button>
          <Button onClick={() => update({ functional: true, analytics: true, marketing: true })}>
            Accept all
          </Button>
        </div>
        <Button variant="primary" onClick={() => update(preferences)}>
          Save preferences
        </Button>
      </div>
    </>
  );
}

export function CookieConsentDrawer() {
  const { isOpen, close } = use(CookieConsentContext);

  return (
    <Drawer open={isOpen} onOpenChange={(open) => !open && close()}>
      <DrawerPanel size="lg">
        <CookieConsentForm />
      </DrawerPanel>
    </Drawer>
  );
}
