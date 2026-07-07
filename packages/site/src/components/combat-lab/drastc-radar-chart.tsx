import type { DrastcRadarDatum } from "@/lib/combat-lab/chart";

const CHART_SIZE = 320;
const CHART_CENTER = CHART_SIZE / 2;
const CHART_RADIUS = 112;
const MAX_SCORE = 10;
const GRID_LEVELS = [2, 4, 6, 8, 10];

type DrastcRadarChartProps = {
  data: DrastcRadarDatum[];
};

export function DrastcRadarChart({ data }: DrastcRadarChartProps) {
  const axisPoints = data.map((item, index) => ({
    ...item,
    ...getRadarPoint(index, data.length, CHART_RADIUS),
    label: getRadarPoint(index, data.length, CHART_RADIUS + 26),
  }));
  const scorePoints = axisPoints
    .map((point) => {
      const scoreRadius =
        (Math.max(0, Math.min(MAX_SCORE, point.score)) / MAX_SCORE) * CHART_RADIUS;
      const scorePoint = getRadarPoint(point.index, data.length, scoreRadius);
      return `${scorePoint.x},${scorePoint.y}`;
    })
    .join(" ");

  return (
    <div className="h-80 min-h-80">
      <svg
        aria-label="DRASTC score radar chart"
        className="h-full w-full overflow-visible text-zinc-700 dark:text-zinc-200"
        role="img"
        viewBox={`0 0 ${CHART_SIZE} ${CHART_SIZE}`}
      >
        <title>DRASTC score radar chart</title>
        {GRID_LEVELS.map((level) => (
          <polygon
            key={level}
            points={axisPoints
              .map((point) => {
                const gridPoint = getRadarPoint(
                  point.index,
                  data.length,
                  (level / MAX_SCORE) * CHART_RADIUS
                );
                return `${gridPoint.x},${gridPoint.y}`;
              })
              .join(" ")}
            className="fill-none stroke-zinc-300 dark:stroke-zinc-700"
            strokeWidth="1"
          />
        ))}
        {axisPoints.map((point) => (
          <line
            key={point.axis}
            x1={CHART_CENTER}
            x2={point.x}
            y1={CHART_CENTER}
            y2={point.y}
            className="stroke-zinc-200 dark:stroke-zinc-800"
            strokeWidth="1"
          />
        ))}
        <polygon
          points={scorePoints}
          className="fill-blue-600/30 stroke-blue-600"
          strokeLinejoin="round"
          strokeWidth="2"
        />
        {axisPoints.map((point) => (
          <text
            key={point.axis}
            x={point.label.x}
            y={point.label.y}
            className="fill-current font-bold text-[13px]"
            dominantBaseline="middle"
            textAnchor="middle"
          >
            {point.axis}
            <title>{`${point.fullName}: ${point.score}`}</title>
          </text>
        ))}
      </svg>
    </div>
  );
}

function getRadarPoint(
  index: number,
  total: number,
  radius: number
): {
  index: number;
  x: number;
  y: number;
} {
  const angle = -Math.PI / 2 + (index / total) * Math.PI * 2;

  return {
    index,
    x: round(CHART_CENTER + Math.cos(angle) * radius),
    y: round(CHART_CENTER + Math.sin(angle) * radius),
  };
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}
