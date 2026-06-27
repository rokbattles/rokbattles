import { useCallback, useEffect, useState } from "react";

import type { Banner, BannerType } from "../lib/banner.ts";

const bannerDurationMs = 3000;

type UseBannerResult = {
  banner: Banner | null;
  showBanner: (type: BannerType, message: string) => void;
};

export function useBanner(): UseBannerResult {
  const [banner, setBanner] = useState<Banner | null>(null);

  useEffect(() => {
    if (!banner) {
      return;
    }

    const timeoutId = window.setTimeout(() => {
      setBanner(null);
    }, bannerDurationMs);

    return () => {
      window.clearTimeout(timeoutId);
    };
  }, [banner]);

  const showBanner = useCallback((type: BannerType, message: string) => {
    setBanner({ type, message });
  }, []);

  return { banner, showBanner };
}
