"use client";

import { useExtracted } from "next-intl";
import { useMemo } from "react";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import {
  Drawer,
  DrawerActions,
  DrawerBody,
  DrawerDescription,
  DrawerTitle,
} from "@/components/ui/drawer";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import { formatUtcTime } from "@/lib/datetime";
import { formatAbbreviatedNumber } from "@/lib/numbers";
import type { ExploreBattleReportRow } from "@/lib/types/explore-battle-reports";
import ExploreOverviewSummaryCard from "./explore-overview-summary-card";

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

export default function ExploreReportOverviewDrawer({
  report,
  onClose,
}: {
  report: ExploreBattleReportRow | null;
  onClose: () => void;
}) {
  const t = useExtracted();
  const timelineChart = useMemo(() => {
    if (!report) {
      return null;
    }

    const samples = report.timeline.sampling.filter(
      (sample) =>
        Number.isFinite(sample.tick) &&
        Number.isFinite(sample.count) &&
        sample.count >= 0
    );

    if (samples.length === 0) {
      return null;
    }

    const counts = samples.map((sample) => sample.count);
    const minCount = Math.min(...counts);
    const maxCount = Math.max(...counts);
    const countRange = Math.max(1, maxCount - minCount);
    const xDivisor = Math.max(1, samples.length - 1);
    const axisMaxLabel = formatAbbreviatedNumber(maxCount);
    const axisMidLabel = formatAbbreviatedNumber(
      Math.round((maxCount + minCount) / 2)
    );
    const axisMinLabel = formatAbbreviatedNumber(minCount);
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

    const chartInnerWidth =
      CHART_WIDTH - chartPadding.left - chartPadding.right;
    const chartInnerHeight =
      CHART_HEIGHT - chartPadding.top - chartPadding.bottom;

    const points = samples.map((sample, index) => {
      const x = chartPadding.left + (chartInnerWidth * index) / xDivisor;
      const normalizedY = (sample.count - minCount) / countRange;
      const y =
        chartPadding.top + chartInnerHeight - normalizedY * chartInnerHeight;

      return { x, y };
    });

    const path = points
      .map(
        (point, index) =>
          `${index === 0 ? "M" : "L"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`
      )
      .join(" ");

    return {
      points,
      path,
      chartPadding,
      axisLabels: {
        max: axisMaxLabel,
        mid: axisMidLabel,
        min: axisMinLabel,
      },
      startTime: formatUtcTime(
        report.timeline.startTimestamp ?? report.startTimestamp
      ),
      endTime: formatUtcTime(
        report.timeline.endTimestamp ?? report.endTimestamp
      ),
    };
  }, [report]);

  return (
    <Drawer onClose={onClose} open={report !== null} size="4xl">
      {report ? (
        <>
          <DrawerTitle>{t("Battle overview")}</DrawerTitle>
          <DrawerDescription>
            {t("A quick glance at the battle report before opening it.")}
          </DrawerDescription>
          <DrawerBody className="space-y-6">
            {timelineChart && (
              <section>
                <Subheading className="mb-3">{t("Battle timeline")}</Subheading>
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
                      x1={timelineChart.chartPadding.left}
                      x2={CHART_WIDTH - timelineChart.chartPadding.right}
                      y1={timelineChart.chartPadding.top}
                      y2={timelineChart.chartPadding.top}
                    />
                    <line
                      stroke="currentColor"
                      strokeOpacity="0.12"
                      strokeWidth="1"
                      x1={timelineChart.chartPadding.left}
                      x2={CHART_WIDTH - timelineChart.chartPadding.right}
                      y1={CHART_HEIGHT / 2}
                      y2={CHART_HEIGHT / 2}
                    />
                    <line
                      stroke="currentColor"
                      strokeOpacity="0.12"
                      strokeWidth="1"
                      x1={timelineChart.chartPadding.left}
                      x2={CHART_WIDTH - timelineChart.chartPadding.right}
                      y1={CHART_HEIGHT - timelineChart.chartPadding.bottom}
                      y2={CHART_HEIGHT - timelineChart.chartPadding.bottom}
                    />
                    <path
                      d={timelineChart.path}
                      fill="none"
                      stroke="#0ea5e9"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth="4"
                    />
                    {timelineChart.points.map((point, index) => (
                      <circle
                        cx={point.x}
                        cy={point.y}
                        fill="#38bdf8"
                        key={`sample-${index}-${point.x}-${point.y}`}
                        r={
                          index === 0 ||
                          index === timelineChart.points.length - 1
                            ? 7
                            : 4
                        }
                        stroke="white"
                        strokeWidth="2"
                      />
                    ))}
                    <text
                      fill="currentColor"
                      fontSize="14"
                      textAnchor="start"
                      x={CHART_Y_LABEL_X}
                      y={timelineChart.chartPadding.top + 5}
                    >
                      {timelineChart.axisLabels.max}
                    </text>
                    <text
                      fill="currentColor"
                      fontSize="14"
                      textAnchor="start"
                      x={CHART_Y_LABEL_X}
                      y={CHART_HEIGHT / 2 + 6}
                    >
                      {timelineChart.axisLabels.mid}
                    </text>
                    <text
                      fill="currentColor"
                      fontSize="14"
                      textAnchor="start"
                      x={CHART_Y_LABEL_X}
                      y={CHART_HEIGHT - timelineChart.chartPadding.bottom + 6}
                    >
                      {timelineChart.axisLabels.min}
                    </text>
                  </svg>
                </div>
                <Divider className="my-2" />
                <div className="flex items-center justify-between">
                  <Text>{timelineChart.startTime}</Text>
                  <Text>{timelineChart.endTime}</Text>
                </div>
              </section>
            )}
            <section>
              <Subheading className="mb-3">{t("Data summary")}</Subheading>
              <div className="grid gap-3 lg:grid-cols-2">
                <ExploreOverviewSummaryCard
                  summary={report.summary.sender}
                  title={t("Sender")}
                />
                <ExploreOverviewSummaryCard
                  summary={report.summary.opponent}
                  title={t("All Enemies")}
                />
              </div>
            </section>
          </DrawerBody>
          <DrawerActions>
            <Button onClick={onClose} plain>
              {t("Close")}
            </Button>
            <Button
              color="dark/zinc"
              href={
                report
                  ? `/report/${encodeURIComponent(report.mailId)}`
                  : undefined
              }
            >
              {t("See full report")}
            </Button>
          </DrawerActions>
        </>
      ) : null}
    </Drawer>
  );
}
