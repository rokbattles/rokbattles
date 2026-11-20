export type AccountReportsSource = "account-reports" | "account-rallies" | "account-garrisons";

const sources: Record<string, AccountReportsSource> = {
  "/account/reports": "account-reports",
  "/account/rallies": "account-rallies",
  "/account/garrisons": "account-garrisons",
};

/** The `from` value a report link carries so its back button returns to the right list. */
export function accountReportsSource(pathname: string): AccountReportsSource | undefined {
  return sources[pathname];
}

export const accountReportsBack: Record<AccountReportsSource, { href: string; label: string }> = {
  "account-reports": { href: "/account/reports", label: "Back to My Reports" },
  "account-rallies": { href: "/account/rallies", label: "Back to My Rallies" },
  "account-garrisons": { href: "/account/garrisons", label: "Back to My Garrisons" },
};
