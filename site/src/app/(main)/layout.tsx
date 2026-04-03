import { ThemeProvider } from "@wrksz/themes/next";
import { PlatformLayout } from "@/components/platform-layout";
import PlatformProviders from "@/components/platform-providers";
import { getCurrentUser } from "@/lib/current-user";

export default async function Layout({ children }: LayoutProps<"/">) {
  const user = await getCurrentUser();
  const initialGovernors = user?.claimedGovernors ?? [];
  const initialActiveGovernorId = initialGovernors[0]?.governorId;

  return (
    <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
      <PlatformProviders
        initialGovernors={initialGovernors}
        initialActiveGovernorId={initialActiveGovernorId}
      >
        <PlatformLayout initialUser={user}>{children}</PlatformLayout>
      </PlatformProviders>
    </ThemeProvider>
  );
}
