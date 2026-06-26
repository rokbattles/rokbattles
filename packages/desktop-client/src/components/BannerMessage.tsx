import type { ReactNode } from "react";

import { type Banner, bannerClasses } from "../lib/banner.ts";

type BannerMessageProps = {
  banner: Banner;
};

export function BannerMessage({ banner }: BannerMessageProps): ReactNode {
  return (
    <div className={`mb-4 rounded-md border px-3 py-2 text-sm ${bannerClasses[banner.type]}`}>
      {banner.message}
    </div>
  );
}
