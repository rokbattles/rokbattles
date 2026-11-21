import { Text } from "@/components/ui/text";
import type { AdminApplicationDetail } from "@/lib/(app)/admin-program-applications";

export function ProgramApplicationDetails({
  application,
}: {
  application: AdminApplicationDetail | null | undefined;
}) {
  if (application === undefined) {
    return <Text role="status">Loading application details…</Text>;
  }

  if (!application) {
    return <Text role="alert">Unable to load application details. Close and reopen to retry.</Text>;
  }

  const governor = application.defaultGovernor;
  const governorLabel = governor?.governorName
    ? `${governor.governorName} (${governor.governorId})`
    : governor?.governorId;

  return (
    <>
      <dl className="space-y-3 text-sm">
        <div>
          <dt className="text-zinc-400">Discord ID</dt>
          <dd>{application.discordId}</dd>
        </div>
        <div>
          <dt className="text-zinc-400">Email</dt>
          <dd className="wrap-anywhere">{application.email ?? "Unavailable"}</dd>
        </div>
        <div>
          <dt className="text-zinc-400">Default governor</dt>
          <dd>{governorLabel ?? "None"}</dd>
        </div>
      </dl>
      <dl className="space-y-4 border-t border-white/10 pt-5">
        {application.form.map(({ question, answer }) => (
          <div key={question} className="space-y-1">
            <dt className="text-sm font-semibold">{question}</dt>
            <dd className="whitespace-pre-wrap wrap-anywhere text-sm/6 text-zinc-300">{answer}</dd>
          </div>
        ))}
      </dl>
    </>
  );
}
