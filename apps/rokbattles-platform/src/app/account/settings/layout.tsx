import type React from "react";
import { AccountSettingsNav } from "@/components/account/AccountSettingsNav";
import { Heading } from "@/components/ui/Heading";

export default function Layout({ children }: { children: React.ReactNode }) {
  return (
    <div className="space-y-4">
      <div>
        <Heading>Account Settings</Heading>
      </div>
      <AccountSettingsNav />
      {children}
    </div>
  );
}
