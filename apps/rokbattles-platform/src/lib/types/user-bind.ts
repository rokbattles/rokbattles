import type { WithId } from "mongodb";

export const USER_BIND_TYPES = ["main", "alt", "farm"] as const;

export type UserBindType = (typeof USER_BIND_TYPES)[number];

export interface UserBindFields {
  discordId: string;
  governorId: number;
  kingdom: number | null;
  appUid: number | null;
  name: string | null;
  avatarUrl: string | null;
  frameUrl: string | null;
  type: UserBindType;
  isDefault: boolean;
  isVisible: boolean;
  pendingDeleteAt?: Date;
  createdAt: Date;
  updatedAt: Date;
}

export type UserBindDocument = WithId<UserBindFields>;

export interface UserBindView {
  id: string;
  governorId: number;
  kingdom: number | null;
  appUid: number | null;
  name: string | null;
  avatarUrl: string | null;
  frameUrl: string | null;
  type: UserBindType;
  isDefault: boolean;
  isVisible: boolean;
  pendingDeleteAt: string | null;
  createdAt: string;
  updatedAt: string;
}
