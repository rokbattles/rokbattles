import { connection } from "next/server";
import { DnsCheck } from "@/components/(marketing)/docs/dns-check";
import { SiteLink } from "@/components/(marketing)/site-link";
import { Heading } from "@/components/(marketing)/ui/heading";

export default async function DnsCheckPage() {
  await connection();

  return (
    <article className="space-y-8 text-base/7 text-zinc-300">
      <header className="space-y-4">
        <Heading level={1}>DNS setup check</Heading>
        <p>Check whether your browser uses ROK Battles DNS on the device you play on.</p>
      </header>
      {process.env.DNS_PROBE_SECRET ? (
        <DnsCheck />
      ) : (
        <p className="rounded-sm border border-white/15 bg-white/5 p-5">
          The DNS check is currently unavailable. You can still review the setup steps below.
        </p>
      )}
      <section aria-labelledby="dns-check-setup" className="space-y-4">
        <h2 id="dns-check-setup" className="text-xl font-medium text-white">
          Before you run the check
        </h2>
        <ul className="list-disc space-y-3 pl-5 marker:text-zinc-500 [&_a]:text-orange-400 [&_a]:underline [&_a]:underline-offset-4">
          <li>
            <SiteLink href="/docs/installation/ios">iPhone and iPad</SiteLink>. Open this page in
            Safari with the current ROK Battles DNS profile installed and enabled.
          </li>
          <li>
            <SiteLink href="/docs/installation/android">Android</SiteLink>. Turn on Intra with the
            ROK Battles server selected. If your browser is selected under Excluded apps, clear its
            checkbox for the test. If Chrome uses a custom Secure DNS provider, turn off Chrome’s
            Secure DNS. Restore your settings after the check.
          </li>
        </ul>
      </section>
    </article>
  );
}
