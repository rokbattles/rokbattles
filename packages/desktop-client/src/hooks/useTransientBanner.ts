import { useCallback, useEffect, useState } from "react";

import type { Banner, BannerType } from "../lib/banner.ts";

const bannerDurationMs = 3000;

type UseTransientBannerResult = {
  banner: Banner | null;
  showTransientBanner: (type: BannerType, message: string) => void;
};

export function useTransientBanner(): UseTransientBannerResult {
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

  const showTransientBanner = useCallback((type: BannerType, message: string) => {
    setBanner({ type, message });
  }, []);

  return { banner, showTransientBanner };
}
