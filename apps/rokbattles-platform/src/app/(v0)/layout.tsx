import type { ReactNode } from "react";
import { PlatformLayout } from "@/components/PlatformLayout";
import PlatformProviders from "@/components/PlatformProviders";

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <PlatformProviders>
      <PlatformLayout>{children}</PlatformLayout>
    </PlatformProviders>
  );
}
