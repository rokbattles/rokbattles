"use client";

import { useExtracted } from "next-intl";
import { BindsClaimForm } from "@/components/account-binds/binds-claim-form";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { MAX_GOVERNOR_BINDS } from "@/lib/account-binds/constants";

type BindsClaimSectionProps = {
  canClaimMore: boolean;
  onClaimed: () => Promise<void>;
};

export function BindsClaimSection({ canClaimMore, onClaimed }: BindsClaimSectionProps) {
  const t = useExtracted();

  return (
    <section className="space-y-4">
      <Subheading level={3}>{t("Bind a governor")}</Subheading>
      <Text>
        {t(
          "Link a governor to your account by entering the Governor ID from Rise of Kingdoms. You can claim up to {value}.",
          { value: MAX_GOVERNOR_BINDS.toString() }
        )}
      </Text>
      <BindsClaimForm canClaimMore={canClaimMore} onClaimed={onClaimed} />
    </section>
  );
}
