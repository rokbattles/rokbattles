export interface SessionDocument {
  sessionId: string;
  userId: string;
  createdAt: Date;
  expiresAt: Date;
}

export interface UserDocument {
  discordId: string;
  username: string;
  discriminator: string;
  globalName: string | null;
  email: string;
  avatar: string | null;
  createdAt?: Date;
  updatedAt?: Date;
}

export interface ClaimedGovernorDocument {
  discordId: string;
  governorId: number;
  createdAt: Date;
  governorName: string | null;
  governorAvatar: string | null;
}
