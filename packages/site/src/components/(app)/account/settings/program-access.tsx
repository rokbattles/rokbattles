"use client";

import { useState } from "react";
import { ProgramApplicationDrawer } from "@/components/(app)/account/settings/program-application-drawer";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useProgramApplications } from "@/hooks/(app)/use-program-applications";
import { PROGRAMS } from "@/lib/(app)/program-applications";
import { hasRole } from "@/lib/(app)/roles";
import type { ProgramApplication, ProgramDefinition } from "@/lib/(app)/types/program-application";
import type { CurrentUser } from "@/lib/types/current-user";

type ProgramAccessProps = {
  user: CurrentUser;
};

export function ProgramAccess({ user }: ProgramAccessProps) {
  const { applications, statusAvailable, loading, refresh, recordApplication } =
    useProgramApplications();
  const [program, setProgram] = useState<ProgramDefinition | null>(null);
  const [notice, setNotice] = useState("");

  function submitted(application: ProgramApplication) {
    recordApplication(application);
    setProgram(null);
    setNotice("Your application has been submitted.");
  }

  return (
    <>
      <dl>
        {PROGRAMS.map((details) => (
          <div key={details.id} className="flex items-center justify-between gap-4 px-3 py-3">
            <dt className="text-sm">{details.label}</dt>
            <dd>
              {hasRole(user, details.id) ? (
                <Badge color="green">Active</Badge>
              ) : applications.some((item) => item.program === details.id) ? (
                <Badge color="orange">Pending submission</Badge>
              ) : (
                <Button
                  outline
                  disabled={!statusAvailable || loading}
                  aria-label={`Apply to ${details.label}`}
                  onClick={() => {
                    setNotice("");
                    setProgram(details);
                  }}
                >
                  Apply
                </Button>
              )}
            </dd>
          </div>
        ))}
      </dl>
      <p role="status" className="px-3 text-sm text-zinc-400">
        {notice}
      </p>
      {program ? (
        <ProgramApplicationDrawer
          program={program}
          onClose={() => setProgram(null)}
          onSubmitted={submitted}
          onConflict={refresh}
        />
      ) : null}
    </>
  );
}
