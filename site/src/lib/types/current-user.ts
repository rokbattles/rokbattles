export interface ClaimedGovernor {
  governorId: number;
  governorName: string | null;
  governorAvatar: string | null;
  default: boolean;
}

export interface CurrentUser {
  discordId: string;
  username: string;
  globalName: string | null;
  email: string;
  avatar: string | null;
  claimedGovernors: ClaimedGovernor[];
}
