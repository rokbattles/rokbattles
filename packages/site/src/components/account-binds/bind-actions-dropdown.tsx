"use client";

import { ChevronDown } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dropdown,
  DropdownButton,
  DropdownItem,
  DropdownLabel,
  DropdownMenu,
} from "@/components/ui/dropdown";
import type { ClaimedGovernor } from "@/lib/types/current-user";

type BindActionsDropdownProps = {
  bind: ClaimedGovernor;
  disabled: boolean;
  onSetDefault: (governorId: number) => Promise<void>;
  onUnlink: (governorId: number) => Promise<void>;
};

export function BindActionsDropdown({
  bind,
  disabled,
  onSetDefault,
  onUnlink,
}: BindActionsDropdownProps) {
  return (
    <Dropdown>
      <DropdownButton as={Button} plain disabled={disabled}>
        Actions
        <ChevronDown data-slot="icon" aria-hidden="true" />
      </DropdownButton>
      <DropdownMenu className="min-w-52" anchor="bottom end">
        <DropdownItem
          disabled={disabled || bind.default}
          onClick={() => {
            void onSetDefault(bind.governorId);
          }}
        >
          <DropdownLabel>Set default bind</DropdownLabel>
        </DropdownItem>
        <DropdownItem
          disabled={disabled}
          onClick={() => {
            void onUnlink(bind.governorId);
          }}
        >
          <DropdownLabel>Unlink bind</DropdownLabel>
        </DropdownItem>
      </DropdownMenu>
    </Dropdown>
  );
}
