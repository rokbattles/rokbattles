type SummaryMetricProps = {
  description: string;
  label: string;
  value: string;
};

export function SummaryMetric({ description, label, value }: SummaryMetricProps) {
  return (
    <div className="flex h-full flex-col border-b pb-4 border-white/10">
      <div className="space-y-1">
        <div className="font-semibold text-sm text-white">{label}</div>
        <p className="text-sm text-zinc-400">{description}</p>
      </div>
      <div className="mt-auto pt-3 font-semibold text-2xl/8 text-white">{value}</div>
    </div>
  );
}
