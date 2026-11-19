"use client";

import { Ellipsis, Search, Share2, Star, Unlink } from "lucide-react";
import { Avatar } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dropdown,
  DropdownButton,
  DropdownDivider,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import type { ClaimedGovernor } from "@/lib/types/current-user";

type BindsListProps = {
  binds: ClaimedGovernor[];
  selected: ClaimedGovernor[];
  disabled: boolean;
  onSelectionChange: (ids: number[]) => void;
  onSetDefault: (id: number) => void;
  onFindMore: (id: number) => void;
  onShare: (binds: ClaimedGovernor[]) => void;
  onUnlink: (binds: ClaimedGovernor[]) => void;
  onManageShares: (id: number) => void;
};

export function BindsList({
  binds,
  selected,
  disabled,
  onSelectionChange,
  onSetDefault,
  onFindMore,
  onShare,
  onUnlink,
  onManageShares,
}: BindsListProps) {
  if (binds.length === 0) {
    return <Text>No governors bound yet.</Text>;
  }

  const selectedIds = new Set(selected.map((bind) => bind.governorId));
  const allSelected = selected.length === binds.length;
  const single = selected.length === 1 ? selected[0] : undefined;

  const updateSelection = (governorId: number, checked: boolean) => {
    onSelectionChange(
      checked ? [...selectedIds, governorId] : [...selectedIds].filter((id) => id !== governorId)
    );
  };

  return (
    <div className="space-y-3" aria-busy={disabled}>
      {selected.length > 0 ? (
        <div className="flex flex-wrap items-center gap-2">
          {single ? (
            <>
              <Button
                outline
                disabled={disabled || single.default}
                onClick={() => onSetDefault(single.governorId)}
              >
                <Star data-slot="icon" aria-hidden="true" />
                Set default
              </Button>
              {!single.isShared ? (
                <Button outline disabled={disabled} onClick={() => onFindMore(single.governorId)}>
                  <Search data-slot="icon" aria-hidden="true" />
                  Find related governors
                </Button>
              ) : null}
            </>
          ) : null}
          <Button
            outline
            disabled={disabled || selected.some((bind) => bind.isShared)}
            onClick={() => onShare(selected)}
          >
            <Share2 data-slot="icon" aria-hidden="true" />
            Share
          </Button>
          <Button outline disabled={disabled} onClick={() => onUnlink(selected)}>
            <Unlink data-slot="icon" aria-hidden="true" />
            Unbind
          </Button>
        </div>
      ) : null}
      <Table dense className="[--gutter:--spacing(0)]">
        <caption className="sr-only">Governor binds</caption>
        <TableHead>
          <TableRow>
            <TableHeader className="w-10">
              <Checkbox
                aria-label="Select all governors"
                checked={allSelected}
                indeterminate={!allSelected && selected.length > 0}
                disabled={disabled}
                onChange={() =>
                  onSelectionChange(allSelected ? [] : binds.map((bind) => bind.governorId))
                }
              />
            </TableHeader>
            <TableHeader scope="col">Governor</TableHeader>
            <TableHeader scope="col" className="hidden sm:table-cell">
              Access
            </TableHeader>
            <TableHeader scope="col" className="w-12 text-right">
              <span className="sr-only">Actions</span>
            </TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {binds.map((bind) => {
            const id = bind.governorId;
            const name = bind.governorName ?? `Governor ${id}`;
            const isSelected = selectedIds.has(id);
            const recipientCount = bind.sharedWith?.length ?? 0;
            return (
              <TableRow
                key={id}
                data-selected={isSelected || undefined}
                className="data-selected:bg-white/5"
              >
                <TableCell>
                  <Checkbox
                    aria-label={`Select ${name}`}
                    checked={isSelected}
                    disabled={disabled}
                    onChange={(checked) => updateSelection(id, checked)}
                  />
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-2 sm:gap-3">
                    <Avatar
                      src={bind.governorAvatar}
                      initials={name[0]}
                      className="size-8 shrink-0 sm:size-9"
                      square
                    />
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-x-2 whitespace-normal font-medium">
                        <span className="min-w-0 wrap-anywhere">{name}</span>
                        {bind.default ? <Badge color="emerald">Default</Badge> : null}
                      </div>
                      <div className="text-xs text-zinc-400">{id}</div>
                      {bind.isShared || recipientCount > 0 ? (
                        <div className="mt-0.5 text-xs text-zinc-400 sm:hidden">
                          {bind.isShared ? "Shared with you" : `Shared with ${recipientCount}`}
                        </div>
                      ) : null}
                    </div>
                  </div>
                </TableCell>
                <TableCell className="hidden sm:table-cell">
                  {bind.isShared ? (
                    <Badge color="sky">Shared with you</Badge>
                  ) : recipientCount > 0 ? (
                    <Button plain onClick={() => onManageShares(id)} disabled={disabled}>
                      Shared with {recipientCount}
                    </Button>
                  ) : (
                    <span className="text-zinc-400">Primary</span>
                  )}
                </TableCell>
                <TableCell className="text-right">
                  <Dropdown>
                    <DropdownButton
                      as={Button}
                      plain
                      disabled={disabled}
                      aria-label={`Actions for ${name}`}
                    >
                      <Ellipsis data-slot="icon" aria-hidden="true" />
                    </DropdownButton>
                    <DropdownMenu className="min-w-48" anchor="bottom end">
                      <DropdownItem onClick={() => onSetDefault(id)} disabled={bind.default}>
                        <DropdownLabel>Set default</DropdownLabel>
                      </DropdownItem>
                      {bind.isShared ? null : (
                        <>
                          <DropdownItem onClick={() => onFindMore(id)}>
                            <DropdownLabel>Find related governors</DropdownLabel>
                          </DropdownItem>
                          <DropdownItem onClick={() => onShare([bind])}>
                            <DropdownLabel>Share</DropdownLabel>
                          </DropdownItem>
                          {recipientCount > 0 ? (
                            <DropdownItem onClick={() => onManageShares(id)}>
                              <DropdownLabel>Manage access</DropdownLabel>
                            </DropdownItem>
                          ) : null}
                        </>
                      )}
                      <DropdownDivider />
                      <DropdownItem onClick={() => onUnlink([bind])}>
                        <DropdownLabel>
                          {bind.isShared ? "Leave shared governor" : "Unbind"}
                        </DropdownLabel>
                      </DropdownItem>
                    </DropdownMenu>
                  </Dropdown>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}
