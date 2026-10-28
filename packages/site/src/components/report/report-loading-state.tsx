"use client";

export function ReportLoadingState() {
  return (
    <>
      <span className="sr-only" role="status" aria-live="polite">
        Loading report...
      </span>
      <div className="space-y-6" aria-hidden="true">
        {[0, 1].map((index) => (
          <div
            key={index}
            className="h-72 animate-pulse rounded-2xl border border-white/10 bg-white/5"
          />
        ))}
      </div>
    </>
  );
}
