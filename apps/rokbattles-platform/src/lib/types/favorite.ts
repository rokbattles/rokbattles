export type FavoriteReportType = "battle" | "duel";

export interface ReportFavoriteDocument {
  discordId: string;
  reportType: FavoriteReportType;
  parentHash: string;
  createdAt: Date;
}
