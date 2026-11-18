import { Check, CircleAlert } from "lucide-react";
import type { Metadata } from "next";
import { cookies } from "next/headers";
import { Button } from "@/components/ui/button";
import { TextLink } from "@/components/ui/text";
import type { AppPermission } from "@/lib/(app)/developer-apps";
import { AccountEmail } from "./account-email";

export const metadata: Metadata = {
  title: "Authorize an app - ROK Battles",
  robots: { index: false, follow: false },
};

type Consent = {
  name: string;
  description: string | null;
  termsUrl: string | null;
  privacyUrl: string | null;
  maskedEmail: string;
  permissions: AppPermission[];
  canAddAndRead: boolean;
};

async function getConsent(request: string): Promise<Consent | null> {
  const cookieStore = await cookies();
  const sid = cookieStore.get("sid")?.value;
  const browser = cookieStore.get("__Host-oidc-browser")?.value;
  if (!sid || !browser) {
    return null;
  }

  const api = process.env.API_URL || "http://localhost:8001";
  const query = new URLSearchParams({ request });
  const response = await fetch(`${api}/v1/auth/authorize/consent?${query}`, {
    headers: { Cookie: `sid=${sid}; __Host-oidc-browser=${browser}` },
    cache: "no-store",
  });

  return response.ok ? ((await response.json()) as Consent) : null;
}

export default async function Page({ searchParams }: PageProps<"/oauth/consent">) {
  const { request } = await searchParams;
  const validRequest = typeof request === "string" && /^[a-zA-Z0-9_-]{43}$/.test(request);
  const consent = validRequest ? await getConsent(request) : null;

  if (!consent || !validRequest) {
    return (
      <section className="space-y-2 px-6 pt-6 pb-8 sm:px-8">
        <h1 className="text-xl font-semibold tracking-tight">Authorization request unavailable</h1>
        <p className="text-sm/6 text-zinc-400">Return to the app and start authorization again.</p>
      </section>
    );
  }

  return (
    <section aria-labelledby="consent-title">
      <div className="space-y-5 px-6 pt-6 pb-6 sm:px-8 sm:pt-7">
        <header className="space-y-2">
          <h1
            id="consent-title"
            className="text-2xl/8 font-semibold wrap-anywhere tracking-tight sm:text-3xl/9"
          >
            Authorize {consent.name}
          </h1>
          {consent.description ? (
            <p className="text-sm/6 wrap-anywhere text-zinc-400">{consent.description}</p>
          ) : null}
          <div className="text-sm/5 text-zinc-400">
            <div className="flex flex-wrap items-baseline gap-x-2">
              <p>Signed in with Discord</p>
              <form action="/proxy/v1/auth/authorize/decision" method="post">
                <input type="hidden" name="request" value={request} />
                <button
                  type="submit"
                  name="decision"
                  value="switch_account"
                  className="min-h-6 cursor-pointer rounded text-white underline decoration-white/50 hover:decoration-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-orange-300"
                >
                  Not you?
                </button>
              </form>
            </div>
            <AccountEmail key={request} request={request} maskedEmail={consent.maskedEmail} />
          </div>
        </header>
        <section
          aria-labelledby="permissions-title"
          className="rounded-xl border border-white/10 bg-black/20 px-4 py-4"
        >
          <h2 id="permissions-title" className="text-sm font-semibold text-zinc-100">
            Requested permissions
          </h2>
          <ul className="mt-3 space-y-2.5">
            {consent.permissions.map((permission) => (
              <li key={permission.id} className="flex items-start gap-2.5 text-sm/5 text-zinc-200">
                <Check aria-hidden="true" className="mt-0.5 size-4 shrink-0 text-orange-400" />
                <span>{permission.description}</span>
              </li>
            ))}
          </ul>
          {consent.canAddAndRead ? (
            <p className="mt-3 flex items-start gap-2.5 border-t border-white/10 pt-3 text-sm/5 text-orange-200">
              <CircleAlert aria-hidden="true" className="mt-0.5 size-4 shrink-0" />
              <span>This app can add a governor bind and read data for that governor.</span>
            </p>
          ) : null}
        </section>
        <div className="space-y-2 text-sm/5 text-zinc-400">
          <p>
            The app can renew access during this session. You can deauthorize it in{" "}
            <TextLink href="/account/settings/apps" target="_blank" rel="noopener noreferrer">
              Account Settings
            </TextLink>
            .
          </p>
          {consent.termsUrl || consent.privacyUrl ? (
            <div className="flex flex-wrap gap-x-4 gap-y-1">
              {consent.termsUrl ? (
                <TextLink href={consent.termsUrl} target="_blank" rel="noopener noreferrer">
                  Terms of Service
                </TextLink>
              ) : null}
              {consent.privacyUrl ? (
                <TextLink href={consent.privacyUrl} target="_blank" rel="noopener noreferrer">
                  Privacy Policy
                </TextLink>
              ) : null}
            </div>
          ) : null}
        </div>
      </div>
      <form
        action="/proxy/v1/auth/authorize/decision"
        method="post"
        className="grid grid-cols-2 gap-3 border-t border-white/10 bg-black/10 px-6 py-4 sm:px-8"
      >
        <input type="hidden" name="request" value={request} />
        <Button
          type="submit"
          name="decision"
          value="deny"
          outline
          className="min-h-11 items-center!"
        >
          Deny
        </Button>
        <Button
          type="submit"
          name="decision"
          value="allow"
          color="orange"
          className="min-h-11 items-center! text-zinc-950!"
        >
          Authorize
        </Button>
      </form>
    </section>
  );
}
