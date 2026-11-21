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

const PORTRAIT_SIZE = 176;

type Font = {
  name: string;
  data: ArrayBuffer;
  weight: 500 | 700;
  style: "normal";
};

/** Fetches just the glyphs we need from Google Fonts. */
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

// Everything the image writes: the header, the labels, the numbers, "VS", the date and the footer.
// There are no player names.
const IMAGE_TEXT =
  "BATTLE REPORT KvK VS DEAD SEVERELY WOUNDED Battle report rokbattles.com 0123456789,:· UTC JanFebMarAprMayJunJulAugSepOctNovDec";

async function loadFonts(): Promise<Font[]> {
  const fonts = await Promise.all([
    loadGoogleFont("Inter", 700, IMAGE_TEXT),
    loadGoogleFont("Inter", 500, IMAGE_TEXT),
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

function Header({ kvk, when }: { kvk: boolean; when: string }) {
  return (
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
        {kvk ? (
          <div
            style={{
              display: "flex",
              boxSizing: "border-box",
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
        {when}
      </div>
    </div>
  );
}

function Portrait({ sprites }: { sprites: string[] }) {
  return (
    <div
      style={{ display: "flex", position: "relative", width: PORTRAIT_SIZE, height: PORTRAIT_SIZE }}
    >
      {sprites.map((src) => (
        // biome-ignore lint/performance/noImgElement: next/og renders plain img elements.
        // biome-ignore lint/a11y/useAltText: decorative layers inside a generated image.
        <img
          key={src}
          src={src}
          width={PORTRAIT_SIZE}
          height={PORTRAIT_SIZE}
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            width: PORTRAIT_SIZE,
            height: PORTRAIT_SIZE,
          }}
        />
      ))}
    </div>
  );
}

function SideColumn({ side }: { side: ReportEmbedSide }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        width: 450,
        gap: 18,
      }}
    >
      {side.commanders.map((sprites) => (
        <Portrait key={sprites.join("|")} sprites={sprites} />
      ))}
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
          fontSize: 22,
          fontWeight: 500,
          color: COLORS.muted,
          letterSpacing: 3,
          textTransform: "uppercase",
        }}
      >
        {label}
      </div>
      <div style={{ display: "flex", alignItems: "center", marginTop: 8 }}>
        <div
          style={{
            display: "flex",
            width: 220,
            fontSize: 40,
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
            height: 28,
            borderRadius: 14,
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
            width: 220,
            justifyContent: "flex-end",
            fontSize: 40,
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

function Footer() {
  return (
    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
      <div style={{ display: "flex", alignItems: "center" }}>
        {/* biome-ignore lint/performance/noImgElement: next/og renders plain img elements. */}
        {/* biome-ignore lint/a11y/useAltText: decorative logo inside a generated image. */}
        <img src={REPORT_EMBED_LOGO} width={48} height={48} style={{ borderRadius: 10 }} />
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
  boxSizing: "border-box" as const,
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
  fontFamily: "Inter",
  color: COLORS.text,
};

function FallbackCard() {
  return (
    <div style={frame}>
      <Header kvk={false} when="" />
      <div style={{ display: "flex", justifyContent: "center", fontSize: 72, fontWeight: 700 }}>
        Battle report
      </div>
      <Footer />
    </div>
  );
}

function ReportCard({ report }: { report: ReportEmbed }) {
  return (
    <div style={frame}>
      <Header kvk={report.kvk} when={formatWhen(report.mailTime)} />

      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <SideColumn side={report.sender} />
        <div style={{ display: "flex", fontSize: 60, fontWeight: 700, color: COLORS.accent }}>
          VS
        </div>
        <SideColumn side={report.opponent} />
      </div>

      <div
        style={{
          display: "flex",
          boxSizing: "border-box",
          flexDirection: "column",
          gap: 18,
          padding: "20px 32px",
          borderRadius: 18,
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

      <Footer />
    </div>
  );
}

/** Renders fully before responding, so an error can be caught and the plain card used instead. */
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
  const [report, fonts] = await Promise.all([getReportEmbed(id), loadFonts()]);

  const png = await renderPng(report ? <ReportCard report={report} /> : <FallbackCard />, fonts);
  if (png) {
    return pngResponse(png);
  }

  const fallback = await renderPng(<FallbackCard />, fonts);
  return fallback
    ? pngResponse(fallback)
    : new Response("Could not render the report image", { status: 500 });
}
