import { ImageResponse } from "next/og";
import type { ReactElement } from "react";

import {
  formatCount,
  getReportEmbed,
  type ReportEmbed,
  type ReportEmbedSide,
} from "@/lib/report-embed";
import { REPORT_EMBED_LOGO } from "@/lib/report-embed-logo";

export const alt = "Battle report";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

const COLORS = {
  background: "#09090b",
  panel: "#18181b",
  border: "#27272a",
  text: "#fafafa",
  muted: "#a1a1aa",
  accent: "#ff8a3d",
  sender: "#3b82f6",
  enemy: "#f87171",
};

type Font = {
  name: string;
  data: ArrayBuffer;
  weight: 500 | 700;
  style: "normal";
};

/** Fetches just the glyphs we need from Google Fonts, so names in any script are drawn. */
async function loadGoogleFont(
  family: string,
  weight: 500 | 700,
  text: string
): Promise<Font | null> {
  try {
    const css = await (
      await fetch(
        `https://fonts.googleapis.com/css2?family=${family.replaceAll(" ", "+")}:wght@${String(weight)}&text=${encodeURIComponent(text)}`
      )
    ).text();
    const url = css.match(/src: url\((.+?)\) format\('(?:opentype|truetype)'\)/)?.[1];
    if (!url) {
      return null;
    }
    const response = await fetch(url);
    if (!response.ok) {
      return null;
    }
    return { name: family, data: await response.arrayBuffer(), weight, style: "normal" };
  } catch {
    return null;
  }
}

async function loadFonts(text: string): Promise<Font[]> {
  const fonts = await Promise.all([
    loadGoogleFont("Inter", 700, text),
    loadGoogleFont("Inter", 500, text),
    loadGoogleFont("Noto Sans SC", 700, text),
    loadGoogleFont("Noto Sans SC", 500, text),
  ]);
  return fonts.filter((font): font is Font => font !== null);
}

function formatWhen(mailTimeMicros: number): string {
  if (!mailTimeMicros) {
    return "";
  }
  const date = new Date(mailTimeMicros / 1000);
  const day = new Intl.DateTimeFormat("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
    timeZone: "UTC",
  }).format(date);
  const time = new Intl.DateTimeFormat("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
    timeZone: "UTC",
  }).format(date);
  return `${day} · ${time} UTC`;
}

function Portrait({ side }: { side: ReportEmbedSide }) {
  return (
    <div style={{ display: "flex", position: "relative", width: 164, height: 164 }}>
      {side.commanderSprites.map((src) => (
        // biome-ignore lint/performance/noImgElement: next/og renders plain img elements.
        // biome-ignore lint/a11y/useAltText: decorative layers inside a generated image.
        <img
          key={src}
          src={src}
          width={164}
          height={164}
          style={{ position: "absolute", top: 0, left: 0, width: 164, height: 164 }}
        />
      ))}
    </div>
  );
}

function SideColumn({ side, extra }: { side: ReportEmbedSide; extra?: string }) {
  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        width: 430,
      }}
    >
      <Portrait side={side} />
      <div style={{ display: "flex", justifyContent: "center", width: 430, marginTop: 12 }}>
        <div
          style={{
            display: "block",
            maxWidth: 430,
            fontSize: 40,
            fontWeight: 700,
            color: COLORS.text,
            overflow: "hidden",
            whiteSpace: "nowrap",
            textOverflow: "ellipsis",
          }}
        >
          {side.name}
        </div>
      </div>
      <div
        style={{
          display: "flex",
          marginTop: 4,
          fontSize: 24,
          fontWeight: 500,
          color: COLORS.muted,
        }}
      >
        {[side.alliance ? `[${side.alliance}]` : null, side.commanderName, extra]
          .filter(Boolean)
          .join("  ·  ")}
      </div>
    </div>
  );
}

function StatRow({ label, sender, enemy }: { label: string; sender: number; enemy: number }) {
  const total = sender + enemy;
  const senderShare = total > 0 ? Math.max(4, Math.min(96, (sender / total) * 100)) : 50;
  return (
    <div style={{ display: "flex", flexDirection: "column", width: "100%" }}>
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          fontSize: 20,
          fontWeight: 500,
          color: COLORS.muted,
          letterSpacing: 2,
          textTransform: "uppercase",
        }}
      >
        {label}
      </div>
      <div style={{ display: "flex", alignItems: "center", marginTop: 6 }}>
        <div
          style={{
            display: "flex",
            width: 190,
            fontSize: 34,
            fontWeight: 700,
            color: COLORS.sender,
          }}
        >
          {formatCount(sender)}
        </div>
        <div
          style={{
            display: "flex",
            flex: 1,
            height: 22,
            borderRadius: 11,
            overflow: "hidden",
            backgroundColor: COLORS.border,
          }}
        >
          <div
            style={{
              display: "flex",
              width: `${String(senderShare)}%`,
              backgroundColor: COLORS.sender,
            }}
          />
          <div style={{ display: "flex", flex: 1, backgroundColor: COLORS.enemy }} />
        </div>
        <div
          style={{
            display: "flex",
            width: 190,
            justifyContent: "flex-end",
            fontSize: 31,
            fontWeight: 700,
            color: COLORS.enemy,
          }}
        >
          {formatCount(enemy)}
        </div>
      </div>
    </div>
  );
}

/** Keeps Latin letters, Chinese and Japanese characters, numbers and punctuation. */
function simplifyName(name: string): string {
  const kept = name
    .replace(
      /[^\p{Script=Latin}\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{N}\p{P}\p{Zs}]/gu,
      ""
    )
    .replace(/\s+/g, " ")
    .trim();
  return kept.length > 0 ? kept : "Unknown";
}

function simplifyReport(report: ReportEmbed): ReportEmbed {
  return {
    ...report,
    sender: { ...report.sender, name: simplifyName(report.sender.name) },
    opponent: { ...report.opponent, name: simplifyName(report.opponent.name) },
  };
}

function Footer({ logo }: { logo: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
      <div style={{ display: "flex", alignItems: "center" }}>
        {/* biome-ignore lint/performance/noImgElement: next/og renders plain img elements. */}
        {/* biome-ignore lint/a11y/useAltText: decorative logo inside a generated image. */}
        <img src={logo} width={48} height={48} style={{ borderRadius: 10 }} />
        <div
          style={{
            display: "flex",
            marginLeft: 14,
            fontSize: 32,
            fontWeight: 700,
            color: COLORS.text,
          }}
        >
          rokbattles
        </div>
      </div>
      <div style={{ display: "flex", fontSize: 22, fontWeight: 500, color: COLORS.muted }}>
        rokbattles.com
      </div>
    </div>
  );
}

const frame = {
  display: "flex",
  flexDirection: "column" as const,
  justifyContent: "space-between",
  width: "100%",
  height: "100%",
  padding: "32px 56px 28px",
  backgroundColor: COLORS.background,
  backgroundImage:
    "radial-gradient(circle at 50% -10%, rgba(255,138,61,0.22) 0%, rgba(255,138,61,0) 55%)",
  borderTop: `6px solid ${COLORS.accent}`,
  fontFamily: "Inter, Noto Sans SC",
  color: COLORS.text,
};

function FallbackCard() {
  return (
    <div style={frame}>
      <div
        style={{
          display: "flex",
          fontSize: 22,
          fontWeight: 700,
          color: COLORS.accent,
          letterSpacing: 4,
        }}
      >
        BATTLE REPORT
      </div>
      <div style={{ display: "flex", fontSize: 64, fontWeight: 700 }}>Battle report</div>
      <Footer logo={REPORT_EMBED_LOGO} />
    </div>
  );
}

function ReportCard({ report }: { report: ReportEmbed }) {
  const extra =
    report.opponentCount > 1 ? `+${String(report.opponentCount - 1)} other enemies` : undefined;

  return (
    <div style={frame}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div style={{ display: "flex", alignItems: "center" }}>
          <div
            style={{
              display: "flex",
              fontSize: 22,
              fontWeight: 700,
              color: COLORS.accent,
              letterSpacing: 4,
            }}
          >
            BATTLE REPORT
          </div>
          {report.kvk ? (
            <div
              style={{
                display: "flex",
                marginLeft: 16,
                padding: "2px 12px",
                borderRadius: 8,
                border: `2px solid ${COLORS.border}`,
                fontSize: 20,
                fontWeight: 700,
                color: COLORS.muted,
              }}
            >
              KvK
            </div>
          ) : null}
        </div>
        <div style={{ display: "flex", fontSize: 22, fontWeight: 500, color: COLORS.muted }}>
          {formatWhen(report.mailTime)}
        </div>
      </div>

      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <SideColumn side={report.sender} />
        <div style={{ display: "flex", fontSize: 54, fontWeight: 700, color: COLORS.accent }}>
          VS
        </div>
        <SideColumn side={report.opponent} extra={extra} />
      </div>

      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 10,
          padding: "12px 28px",
          borderRadius: 16,
          backgroundColor: COLORS.panel,
          border: `2px solid ${COLORS.border}`,
        }}
      >
        <StatRow label="Dead" sender={report.sender.dead} enemy={report.opponent.dead} />
        <StatRow
          label="Severely wounded"
          sender={report.sender.severelyWounded}
          enemy={report.opponent.severelyWounded}
        />
      </div>

      <Footer logo={REPORT_EMBED_LOGO} />
    </div>
  );
}

/** Renders fully before responding, so a font or layout error can be caught and retried. */
async function renderPng(element: ReactElement, fonts: Font[]): Promise<ArrayBuffer | null> {
  try {
    return await new ImageResponse(element, { ...size, fonts }).arrayBuffer();
  } catch {
    return null;
  }
}

function pngResponse(body: ArrayBuffer): Response {
  return new Response(body, {
    headers: {
      "Content-Type": contentType,
      "Cache-Control": "public, max-age=3600, s-maxage=3600",
    },
  });
}

export default async function Image({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const report = await getReportEmbed(id);

  const textFor = (r: ReportEmbed | null) =>
    r
      ? `${r.sender.name}${r.opponent.name}${r.sender.alliance ?? ""}${r.opponent.alliance ?? ""}${r.sender.commanderName ?? ""}${r.opponent.commanderName ?? ""}${formatWhen(r.mailTime)}`
      : "";
  const staticText =
    "BATTLE REPORT KvK VS Dead Severely wounded rokbattles.com rokbattles 0123456789,.[]·:UTC +and other enemies Unknown Battle report…";

  if (report) {
    const first = await renderPng(
      <ReportCard report={report} />,
      await loadFonts(textFor(report) + staticText)
    );
    if (first) {
      return pngResponse(first);
    }

    // Some scripts cannot be drawn. Retry with the names reduced to what the fonts support.
    const simple = simplifyReport(report);
    const second = await renderPng(
      <ReportCard report={simple} />,
      await loadFonts(textFor(simple) + staticText)
    );
    if (second) {
      return pngResponse(second);
    }
  }

  const fallback = await renderPng(<FallbackCard />, await loadFonts(staticText));
  return fallback
    ? pngResponse(fallback)
    : new Response("Could not render the report image", { status: 500 });
}
