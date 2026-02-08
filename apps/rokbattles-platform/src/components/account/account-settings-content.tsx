"use client";

import { ChevronDownIcon } from "@heroicons/react/16/solid";
import { useRouter } from "next/navigation";
import { useExtracted } from "next-intl";
import { useState, useTransition } from "react";
import {
  cancelUnlinkBindAction,
  createBindAction,
  makeDefaultBindAction,
  setBindVisibilityAction,
  unlinkBindAction,
} from "@/actions/binds";
import { GameAvatar } from "@/components/game-avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dropdown,
  DropdownButton,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";
import {
  Description,
  ErrorMessage,
  Field,
  FieldGroup,
  Label,
} from "@/components/ui/fieldset";
import { Heading, Subheading } from "@/components/ui/heading";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import type { UserBindView } from "@/lib/types/user-bind";

interface AccountSettingsContentProps {
  email: string;
  binds: UserBindView[];
}

function formatPendingDeleteAt(timestamp: string | null): string | null {
  if (!timestamp) {
    return null;
  }

  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return null;
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}

export default function AccountSettingsContent({
  email,
  binds,
}: AccountSettingsContentProps) {
  const t = useExtracted();
  const router = useRouter();
  const [governorId, setGovernorId] = useState("");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isPending, startTransition] = useTransition();

  const runAction = (
    action: () => Promise<{ ok: boolean; error?: string }>
  ) => {
    setErrorMessage(null);

    startTransition(async () => {
      const result = await action();
      if (!result.ok) {
        setErrorMessage(
          result.error ?? "Something went wrong. Please try again."
        );
        return;
      }

      router.refresh();
    });
  };

  return (
    <div className="mt-8 space-y-8">
      <div className="space-y-2">
        <Heading>{t("Settings")}</Heading>
        <Text>{t("Manage account details and game binds.")}</Text>
      </div>

      <section className="space-y-4">
        <Subheading level={3}>{t("Account")}</Subheading>
        <FieldGroup>
          <Field>
            <Label htmlFor="account-email">{t("Email")}</Label>
            <Input
              disabled
              id="account-email"
              readOnly
              type="email"
              value={email}
            />
            <Description>
              {t("Your email address is synced from Discord.")}
            </Description>
          </Field>
        </FieldGroup>
      </section>

      <section className="space-y-4" id="binds">
        <Subheading level={3}>{t("Game Binds")}</Subheading>

        <form
          className="space-y-4"
          id="bind-form"
          onSubmit={(event) => {
            event.preventDefault();
            runAction(async () => createBindAction({ governorId }));
          }}
        >
          <FieldGroup>
            <Field>
              <Label htmlFor="bind-governor-id">{t("Governor ID")}</Label>
              <Input
                autoComplete="off"
                disabled={isPending}
                id="bind-governor-id"
                inputMode="numeric"
                name="governorId"
                onChange={(event) => setGovernorId(event.target.value)}
                pattern="[0-9]*"
                placeholder="71738515"
                value={governorId}
              />
              {errorMessage ? (
                <ErrorMessage>{errorMessage}</ErrorMessage>
              ) : null}
            </Field>
          </FieldGroup>
          <Button
            disabled={isPending || governorId.trim().length === 0}
            type="submit"
          >
            {isPending ? t("Binding...") : t("Bind account")}
          </Button>
        </form>

        <div className="space-y-2">
          {binds.length > 0 ? (
            <Table bleed>
              <TableHead>
                <TableRow>
                  <TableHeader className="w-8">{t("#")}</TableHeader>
                  <TableHeader>{t("Account")}</TableHeader>
                  <TableHeader className="w-32">{t("ID")}</TableHeader>
                  <TableHeader className="w-32">
                    <span className="sr-only">{t("Actions")}</span>
                  </TableHeader>
                </TableRow>
              </TableHead>
              <TableBody>
                {
                  // biome-ignore lint/complexity/noExcessiveCognitiveComplexity: This row keeps related dropdown state and actions together for maintainability.
                  binds.map((bind, index) => {
                    const displayName = bind.name ?? bind.governorId.toString();
                    const pendingDeleteAt = formatPendingDeleteAt(
                      bind.pendingDeleteAt
                    );
                    const canUnlink = bind.isDefault || bind.appUid == null;

                    return (
                      <TableRow key={bind.id}>
                        <TableCell>{index + 1}</TableCell>
                        <TableCell>
                          <div className="flex items-center gap-3">
                            <GameAvatar
                              alt={displayName}
                              avatarUrl={bind.avatarUrl}
                              className="size-9"
                              frameUrl={bind.frameUrl}
                              initials="G"
                            />
                            <div className="flex min-w-0 flex-wrap items-center gap-2">
                              <p className="truncate font-medium text-zinc-950 dark:text-white">
                                {displayName}
                              </p>
                              <div className="flex flex-wrap items-center gap-2">
                                {bind.isDefault ? (
                                  <Badge color="green">{t("default")}</Badge>
                                ) : null}
                                {bind.isVisible ? null : (
                                  <Badge color="zinc">{t("hidden")}</Badge>
                                )}
                                {pendingDeleteAt ? (
                                  <Badge color="amber">
                                    {t("deletes at {date}", {
                                      date: pendingDeleteAt,
                                    })}
                                  </Badge>
                                ) : null}
                              </div>
                            </div>
                          </div>
                        </TableCell>
                        <TableCell>{bind.governorId}</TableCell>
                        <TableCell>
                          <Dropdown>
                            <DropdownButton
                              as={Button}
                              disabled={isPending}
                              outline
                            >
                              {t("Actions")}
                              <ChevronDownIcon />
                            </DropdownButton>
                            <DropdownMenu
                              anchor="bottom end"
                              className="min-w-48"
                            >
                              {bind.pendingDeleteAt ? (
                                <DropdownItem
                                  onClick={() =>
                                    runAction(async () =>
                                      cancelUnlinkBindAction({
                                        bindId: bind.id,
                                      })
                                    )
                                  }
                                >
                                  <DropdownLabel>
                                    {t("Cancel unlink")}
                                  </DropdownLabel>
                                </DropdownItem>
                              ) : (
                                <>
                                  {bind.isDefault ? null : (
                                    <DropdownItem
                                      onClick={() =>
                                        runAction(async () =>
                                          makeDefaultBindAction({
                                            bindId: bind.id,
                                          })
                                        )
                                      }
                                    >
                                      <DropdownLabel>
                                        {t("Set default")}
                                      </DropdownLabel>
                                    </DropdownItem>
                                  )}
                                  {bind.isDefault ? null : (
                                    <DropdownItem
                                      onClick={() =>
                                        runAction(async () =>
                                          setBindVisibilityAction({
                                            bindId: bind.id,
                                            visible: !bind.isVisible,
                                          })
                                        )
                                      }
                                    >
                                      <DropdownLabel>
                                        {bind.isVisible ? t("Hide") : t("Show")}
                                      </DropdownLabel>
                                    </DropdownItem>
                                  )}
                                  {canUnlink ? (
                                    <DropdownItem
                                      onClick={() =>
                                        runAction(async () =>
                                          unlinkBindAction({ bindId: bind.id })
                                        )
                                      }
                                    >
                                      <DropdownLabel>
                                        {t("Unlink")}
                                      </DropdownLabel>
                                    </DropdownItem>
                                  ) : null}
                                </>
                              )}
                            </DropdownMenu>
                          </Dropdown>
                        </TableCell>
                      </TableRow>
                    );
                  })
                }
              </TableBody>
            </Table>
          ) : null}
        </div>
      </section>
    </div>
  );
}
