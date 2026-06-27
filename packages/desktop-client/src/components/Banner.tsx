import type { ReactNode } from "react";

import { type Banner as BannerContent, bannerClasses } from "../lib/banner.ts";

type BannerProps = {
  banner: BannerContent;
};

export function Banner({ banner }: BannerProps): ReactNode {
  return (
    <div className={`mb-4 rounded-md border px-3 py-2 text-sm ${bannerClasses[banner.type]}`}>
      {banner.message}
    </div>
  );
}
