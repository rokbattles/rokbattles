export type BannerType = "success" | "info" | "error";

export type Banner = {
  type: BannerType;
  message: string;
};

export const bannerClasses: Record<BannerType, string> = {
  success: "border-emerald-700 bg-emerald-950/70 text-emerald-200",
  info: "border-sky-700 bg-sky-950/70 text-sky-200",
  error: "border-rose-700 bg-rose-950/70 text-rose-200",
};
