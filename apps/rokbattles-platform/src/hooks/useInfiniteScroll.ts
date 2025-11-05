import { useCallback, useEffect, useRef } from "react";

type Options = {
  enabled: boolean;
  loading: boolean;
  onLoadMore: () => void;
  rootMargin?: string;
  threshold?: number;
};

export function useInfiniteScroll({
  enabled,
  loading,
  onLoadMore,
  rootMargin = "256px 0px 0px 0px",
  threshold = 0.01,
}: Options) {
  const observerRef = useRef<IntersectionObserver | null>(null);
  const onLoadMoreRef = useRef(onLoadMore);
  onLoadMoreRef.current = onLoadMore;

  useEffect(() => {
    return () => {
      observerRef.current?.disconnect();
      observerRef.current = null;
    };
  }, []);

  const setRef = useCallback(
    (node: Element | null) => {
      // disconnect previous
      observerRef.current?.disconnect();
      observerRef.current = null;

      if (!node || !enabled) return;

      observerRef.current = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (entry.isIntersecting && !loading) {
              onLoadMoreRef.current();
              break;
            }
          }
        },
        { rootMargin, threshold }
      );

      observerRef.current.observe(node);
    },
    [enabled, loading, rootMargin, threshold]
  );

  return setRef;
}
