export interface ClaimedGovernor {
  governorId: number;
  governorName: string | null;
  governorAvatar: string | null;
  default: boolean;
}

export interface CurrentUser {
  discordId: string;
  email: string;
  claimedGovernors: ClaimedGovernor[];
}
