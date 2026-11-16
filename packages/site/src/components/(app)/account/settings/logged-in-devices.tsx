"use client";

import { AppWindow, CircleHelp, LoaderCircle, Monitor, X } from "lucide-react";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { SettingsActionRow } from "@/components/(app)/account/settings/settings-action-row";
import { SettingsEmptyState } from "@/components/(app)/account/settings/settings-empty-state";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import { Text } from "@/components/ui/text";
import { useSessions } from "@/hooks/(app)/use-sessions";
import {
  type DeviceSession,
  formatSessionAge,
  logoutAllSessions,
  logoutSession,
  SessionRequestError,
} from "@/lib/(app)/sessions";

type SessionRowProps = {
  session: DeviceSession;
  now: number | null;
  disabled?: boolean;
  pending?: boolean;
  onLogout?: () => void;
};

function SessionRow({ session, now, disabled, pending, onLogout }: SessionRowProps) {
  const label =
    session.platform === "browser"
      ? "Browser"
      : session.platform === "desktop"
        ? "Desktop Client"
        : "Unknown";
  const Icon =
    session.platform === "browser"
      ? AppWindow
      : session.platform === "desktop"
        ? Monitor
        : CircleHelp;

  return (
    <div className="flex items-center gap-3 py-3">
      <div className="flex size-11 shrink-0 items-center justify-center rounded-lg bg-white/5">
        <Icon aria-hidden="true" className="size-5 text-zinc-300" />
      </div>
      <div className="min-w-0 flex-1 space-y-1">
        <p className="text-sm font-medium">{label}</p>
        {!session.current && now !== null ? (
          <Text className="text-xs!">
            <time
              dateTime={new Date(session.createdAt).toISOString()}
              title={new Date(session.createdAt).toUTCString()}
            >
              Signed in {formatSessionAge(session.createdAt, now)}
            </time>
          </Text>
        ) : null}
      </div>
      {onLogout ? (
        <Button
          plain
          disabled={disabled}
          onClick={onLogout}
          aria-label={`Log out ${label} session`}
        >
          {pending ? (
            <LoaderCircle aria-hidden="true" className="size-5 animate-spin" />
          ) : (
            <X aria-hidden="true" className="size-5" />
          )}
        </Button>
      ) : null}
    </div>
  );
}

export function LoggedInDevices() {
  const router = useRouter();
  const { sessions, error, loading, refresh } = useSessions();
  const [pending, setPending] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [now, setNow] = useState<number | null>(null);

  useEffect(() => {
    setNow(Date.now());
    const timer = setInterval(() => setNow(Date.now()), 60_000);

    return () => clearInterval(timer);
  }, []);

  const currentSession = sessions?.find((session) => session.current);
  const otherSessions = sessions?.filter((session) => !session.current) ?? [];
  const disabled = pending !== null || loading;

  const handleLogout = async (session: DeviceSession) => {
    setPending(session.id);
    setActionError(null);

    try {
      await logoutSession(session.id);
      refresh();
      router.refresh();
    } catch (error) {
      if (error instanceof SessionRequestError && error.status === 401) {
        window.location.replace("/");
        return;
      }

      setActionError("Unable to log out this device. Please try again.");
    } finally {
      setPending(null);
    }
  };

  const handleLogoutAll = async () => {
    setPending("all");
    setActionError(null);

    try {
      await logoutAllSessions();
      window.location.replace("/");
    } catch (error) {
      if (error instanceof SessionRequestError && error.status === 401) {
        window.location.replace("/");
        return;
      }

      setActionError("Unable to log out your devices. Please try again.");
      setPending(null);
    }
  };

  return (
    <>
      {error ? (
        <div role="alert" className="space-y-3">
          <Text>{error}</Text>
          <Button outline onClick={refresh} disabled={loading}>
            Try again
          </Button>
        </div>
      ) : null}
      {!sessions && loading ? <Text role="status">Loading logged-in devices…</Text> : null}
      {sessions ? (
        <>
          {currentSession ? (
            <SettingsSection title="Current Device">
              <SessionRow session={currentSession} now={now} />
            </SettingsSection>
          ) : null}
          <Divider />
          <SettingsSection title="Other Devices">
            {otherSessions.length === 0 ? (
              <SettingsEmptyState>No other devices.</SettingsEmptyState>
            ) : (
              <ul>
                {otherSessions.map((session) => (
                  <li key={session.id}>
                    <SessionRow
                      session={session}
                      now={now}
                      disabled={disabled}
                      pending={pending === session.id}
                      onLogout={() => void handleLogout(session)}
                    />
                  </li>
                ))}
              </ul>
            )}
          </SettingsSection>
        </>
      ) : null}
      <Divider />
      {actionError ? (
        <Text role="alert" className="text-red-400">
          {actionError}
        </Text>
      ) : null}
      <SettingsActionRow
        label="Log out of all known devices"
        description="You’ll have to log back in on all logged out devices."
      >
        <Button color="red" disabled={disabled} onClick={() => void handleLogoutAll()}>
          {pending === "all" ? "Logging out…" : "Log out all known devices"}
        </Button>
      </SettingsActionRow>
    </>
  );
}
