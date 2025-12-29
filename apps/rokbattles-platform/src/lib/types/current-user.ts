export interface ClaimedGovernor {
  governorId: number;
  governorName: string | null;
  governorAvatar: string | null;
}

export interface CurrentUser {
  username: string;
  discriminator: string;
  globalName: string | null;
  email: string;
  avatar: string | null;
  claimedGovernors: ClaimedGovernor[];
}
