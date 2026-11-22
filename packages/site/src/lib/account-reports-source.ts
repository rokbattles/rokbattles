export const openFieldReportLink = {
  source: "account-reports",
  href: "/account/reports",
  label: "Open Field",
} as const;

export const accountReportLinks = [
  openFieldReportLink,
  { source: "account-swarming", href: "/account/swarming", label: "Swarming" },
  { source: "account-rallies", href: "/account/rallies", label: "Rally" },
  { source: "account-garrisons", href: "/account/garrisons", label: "Garrison" },
] as const;

/** Preserves the list's filters and pagination in the report link. */
export function buildBattleReportHref(mailId: string, pathname: string, search: string) {
  const list = accountReportLinks.find(({ href }) => href === pathname);
  const query = new URLSearchParams(search);
  query.set("from", list?.source ?? "reports");

  return `/report/${encodeURIComponent(mailId)}?${query.toString()}`;
}

export function accountReportLinkFrom(source: string | undefined) {
  // Older links use `my-reports` for the account reports list.
  if (source === "my-reports") {
    return openFieldReportLink;
  }

  return accountReportLinks.find((link) => link.source === source);
}
