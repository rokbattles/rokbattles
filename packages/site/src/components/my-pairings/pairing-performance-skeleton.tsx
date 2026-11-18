export function PerformanceValueSkeleton({ children = "000.0M" }: { children?: string }) {
  return (
    <span className="relative inline-block text-transparent" aria-hidden="true">
      {children}
      <span className="absolute inset-x-0 inset-y-1 rounded-sm bg-white/10 motion-safe:animate-pulse" />
    </span>
  );
}

export function PerformanceTimelineSkeleton() {
  return (
    <div className="relative size-full motion-safe:animate-pulse" aria-hidden="true">
      <div className="absolute top-2 right-3 bottom-8 left-[58px] flex flex-col justify-between">
        {[0, 1, 2, 3, 4].map((tick) => (
          <div key={tick} className="relative h-px bg-white/5">
            <span className="absolute -top-1 -left-10 h-2 w-6 rounded-sm bg-white/10" />
          </div>
        ))}
      </div>
      <div className="absolute right-3 bottom-1 left-[58px] flex justify-between">
        {[0, 1, 2, 3, 4].map((tick) => (
          <span key={tick} className="h-2 w-7 rounded-sm bg-white/10" />
        ))}
      </div>
    </div>
  );
}
