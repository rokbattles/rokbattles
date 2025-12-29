import type { ReactNode } from "react";
import { PlatformLayout } from "@/components/PlatformLayout";
import PlatformProviders from "@/components/PlatformProviders";
import { getCurrentUser } from "@/lib/current-user";

export default async function Layout({ children }: { children: ReactNode }) {
  const user = await getCurrentUser();
  const initialGovernors = user?.claimedGovernors ?? [];
  const initialActiveGovernorId = initialGovernors[0]?.governorId;

  return (
    <PlatformProviders
      initialGovernors={initialGovernors}
      initialActiveGovernorId={initialActiveGovernorId}
    >
      <PlatformLayout initialUser={user}>{children}</PlatformLayout>
    </PlatformProviders>
  );
}
