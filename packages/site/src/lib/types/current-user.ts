export type UserRole = "user" | "premium" | "content_creator" | "developer_program" | "admin";

export interface ClaimedGovernor {
  governorId: number;
  governorName: string | null;
  governorAvatar: string | null;
  default: boolean;
}

export interface CurrentUser {
  discordId: string;
  email: string;
  roles: UserRole[];
  claimedGovernors: ClaimedGovernor[];
}
