"use client";

import { Divider } from "@/components/ui/divider";
import { Text } from "@/components/ui/text";
import { formatUtcDateTime } from "@/lib/datetime";
import type { ReportsTimelineSample } from "@/lib/types/reports-list";

const CHART_WIDTH = 960;
const CHART_HEIGHT = 280;
const CHART_BASE_PADDING = {
  top: 24,
  right: 16,
  bottom: 24,
  left: 80,
};
const CHART_Y_LABEL_X = 0;
const CHART_LEFT_LABEL_CHAR_WIDTH = 8;
const CHART_LEFT_LABEL_GUTTER = 18;
const CHART_LEFT_PADDING_MAX = 176;

const numberFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 0,
  notation: "compact",
});

type ReportsOverviewTimelineChartProps = {
  startTimestamp: number;
  endTimestamp: number;
  sampling: ReportsTimelineSample[];
};

export default function ReportsOverviewTimelineChart({
  startTimestamp,
  endTimestamp,
  sampling,
}: ReportsOverviewTimelineChartProps) {
  const validSamples = sampling.filter(
    (sample) => Number.isFinite(sample.tick) && Number.isFinite(sample.count) && sample.count >= 0
  );

  if (validSamples.length === 0) {
    return null;
  }

  const counts = validSamples.map((sample) => sample.count);
  const minCount = Math.min(...counts);
  const maxCount = Math.max(...counts);
  const countRange = Math.max(1, maxCount - minCount);
  const xDivisor = Math.max(1, validSamples.length - 1);
  const axisMaxLabel = numberFormatter.format(maxCount);
  const axisMidLabel = numberFormatter.format(Math.round((maxCount + minCount) / 2));
  const axisMinLabel = numberFormatter.format(minCount);
  const longestAxisLabelLength = Math.max(
    axisMaxLabel.length,
    axisMidLabel.length,
    axisMinLabel.length
  );

  const dynamicLeftPadding = Math.min(
    CHART_LEFT_PADDING_MAX,
    Math.max(
      CHART_BASE_PADDING.left,
      CHART_Y_LABEL_X +
        longestAxisLabelLength * CHART_LEFT_LABEL_CHAR_WIDTH +
        CHART_LEFT_LABEL_GUTTER
    )
  );

  const chartPadding = {
    ...CHART_BASE_PADDING,
    left: dynamicLeftPadding,
  };

  const chartInnerWidth = CHART_WIDTH - chartPadding.left - chartPadding.right;
  const chartInnerHeight = CHART_HEIGHT - chartPadding.top - chartPadding.bottom;

  const points = validSamples.map((sample, index) => {
    const x = chartPadding.left + (chartInnerWidth * index) / xDivisor;
    const normalizedY = (sample.count - minCount) / countRange;
    const y = chartPadding.top + chartInnerHeight - normalizedY * chartInnerHeight;

    return { x, y };
  });

  const path = points
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`)
    .join(" ");

  return (
    <section>
      <div className="h-64 w-full text-base/6 text-zinc-500 sm:text-sm/6 dark:text-zinc-400">
        <svg
          aria-label="Battle timeline graph"
          className="h-full w-full"
          preserveAspectRatio="none"
          role="img"
          viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
        >
          <line
            stroke="currentColor"
            strokeOpacity="0.12"
            strokeWidth="1"
            x1={chartPadding.left}
            x2={CHART_WIDTH - chartPadding.right}
            y1={chartPadding.top}
            y2={chartPadding.top}
          />
          <line
            stroke="currentColor"
            strokeOpacity="0.12"
            strokeWidth="1"
            x1={chartPadding.left}
            x2={CHART_WIDTH - chartPadding.right}
            y1={CHART_HEIGHT / 2}
            y2={CHART_HEIGHT / 2}
          />
          <line
            stroke="currentColor"
            strokeOpacity="0.12"
            strokeWidth="1"
            x1={chartPadding.left}
            x2={CHART_WIDTH - chartPadding.right}
            y1={CHART_HEIGHT - chartPadding.bottom}
            y2={CHART_HEIGHT - chartPadding.bottom}
          />
          <path
            d={path}
            fill="none"
            stroke="#0ea5e9"
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth="4"
          />
          {points.map((point, index) => (
            <circle
              cx={point.x}
              cy={point.y}
              fill="#38bdf8"
              key={`sample-${index}-${point.x}-${point.y}`}
              r={index === 0 || index === points.length - 1 ? 7 : 4}
              stroke="white"
              strokeWidth="2"
            />
          ))}
          <text
            fill="currentColor"
            fontSize="14"
            textAnchor="start"
            x={CHART_Y_LABEL_X}
            y={chartPadding.top + 5}
          >
            {axisMaxLabel}
          </text>
          <text
            fill="currentColor"
            fontSize="14"
            textAnchor="start"
            x={CHART_Y_LABEL_X}
            y={CHART_HEIGHT / 2 + 6}
          >
            {axisMidLabel}
          </text>
          <text
            fill="currentColor"
            fontSize="14"
            textAnchor="start"
            x={CHART_Y_LABEL_X}
            y={CHART_HEIGHT - chartPadding.bottom + 6}
          >
            {axisMinLabel}
          </text>
        </svg>
      </div>
      <Divider className="my-2" />
      <div className="flex items-center justify-between">
        <Text>{formatUtcDateTime(startTimestamp)}</Text>
        <Text>{formatUtcDateTime(endTimestamp)}</Text>
      </div>
    </section>
  );
}
