"use client";

import { useWindowVirtualizer } from "@tanstack/react-virtual";

import { memo, useEffect, useLayoutEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import { ReportEntryCard } from "@/components/report/report-entry-card";
import { Divider } from "@/components/ui/divider";
import type { ReportEntry } from "@/lib/types/report";

export const ReportEntryList = memo(function ReportEntryList({
  entries,
}: {
  entries: ReportEntry[];
}) {
  const listRef = useRef<HTMLOListElement>(null);
  const [scrollMargin, setScrollMargin] = useState(0);
  const [printing, setPrinting] = useState(false);

  // Small reports do not need virtualization
  const virtualized = entries.length > 10 && !printing;

  const virtualizer = useWindowVirtualizer({
    count: entries.length,
    estimateSize: () => 1800,
    overscan: 1,
    scrollMargin,
    enabled: virtualized,
  });

  useLayoutEffect(() => {
    const list = listRef.current;

    if (!list || !virtualized) return;

    let width = list.clientWidth;

    const updateLayout = () => {
      setScrollMargin(list.getBoundingClientRect().top + window.scrollY);

      // Cards become taller when participants stack on mobile. Discard cached
      // heights on width changes, including a desktop/mobile breakpoint change.
      if (width !== list.clientWidth) {
        width = list.clientWidth;
        virtualizer.measure();
      }
    };

    updateLayout();

    const observer = new ResizeObserver(updateLayout);
    observer.observe(list);

    if (list.parentElement) observer.observe(list.parentElement);

    window.addEventListener("resize", updateLayout);

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", updateLayout);
    };
  }, [virtualized, virtualizer]);

  useEffect(() => {
    const beforePrint = () => flushSync(() => setPrinting(true));
    const afterPrint = () => setPrinting(false);

    window.addEventListener("beforeprint", beforePrint);
    window.addEventListener("afterprint", afterPrint);

    return () => {
      window.removeEventListener("beforeprint", beforePrint);
      window.removeEventListener("afterprint", afterPrint);
    };
  }, []);

  const items = virtualized
    ? virtualizer.getVirtualItems()
    : entries.map((_, index) => ({ index, key: index, start: 0 }));

  return (
    <ol
      ref={listRef}
      aria-label="Battle Reports"
      className="relative list-none"
      style={
        virtualized ? { height: virtualizer.getTotalSize(), overflowAnchor: "none" } : undefined
      }
    >
      {items.map((item) => (
        <li
          key={item.key}
          ref={virtualized ? virtualizer.measureElement : undefined}
          data-index={item.index}
          aria-posinset={item.index + 1}
          aria-setsize={entries.length}
          className={item.index < entries.length - 1 ? "pb-12" : undefined}
          style={
            virtualized
              ? {
                  position: "absolute",
                  top: 0,
                  left: 0,
                  width: "100%",
                  transform: `translateY(${item.start - scrollMargin}px)`,
                }
              : undefined
          }
        >
          <ReportEntryCard entry={entries[item.index]} />

          {item.index < entries.length - 1 ? <Divider className="mt-8" /> : null}
        </li>
      ))}
    </ol>
  );
});
