import { ImageResponse } from "@takumi-rs/image-response";
import type { Font } from "@takumi-rs/core";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { palette } from "./colors";
import type { CSSProperties, ReactNode } from "react";

const abs: CSSProperties = { display: "flex", position: "absolute" };
const serif: CSSProperties = { fontFamily: "DM Serif Display" };

let fontData: Buffer | null = null;

function loadFont(): Buffer {
  if (!fontData) {
    fontData = readFileSync(
      join(process.cwd(), "public/dm-serif-display-latin-400-normal.woff2"),
    );
  }
  return fontData;
}

export function getOGFont(): Font {
  return {
    name: "DM Serif Display",
    data: loadFont(),
    weight: 400,
    style: "normal",
  };
}

export type OGResponseOptions = {
  width?: number;
  height?: number;
  format?: "png" | "webp" | "jpeg";
};

export function createOGResponse(
  element: React.ReactElement,
  options?: OGResponseOptions,
) {
  const { width = 1200, height = 630, format = "png" } = options ?? {};

  return new ImageResponse(element, {
    width,
    height,
    format,
    fonts: [getOGFont()],
  });
}

function DotGrid() {
  const dots: ReactNode[] = [];
  const cols = 24;
  const rows = 12;
  const spacingX = 1200 / cols;
  const spacingY = 630 / rows;

  for (let row = 0; row < rows; row++) {
    for (let col = 0; col < cols; col++) {
      dots.push(
        <div
          key={`${row}-${col}`}
          style={{
            ...abs,
            left: `${col * spacingX + spacingX / 2}px`,
            top: `${row * spacingY + spacingY / 2}px`,
            width: "1.5px",
            height: "1.5px",
            borderRadius: "50%",
            background: `${palette.muted}18`,
          }}
        />,
      );
    }
  }

  return <>{dots}</>;
}

function FunnelWatermark() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      fill="none"
      width="280"
      height="280"
      style={{
        position: "absolute",
        right: "-20px",
        top: "50%",
        transform: "translateY(-50%)",
        opacity: 0.04,
      }}
    >
      <path
        d="M8 8h16l-5.5 12h-5L8 8z M22.625 11L27 8"
        stroke={palette.accent}
        strokeWidth="1.75"
        strokeLinejoin="miter"
        fill="none"
      />
      <path d="M16 20v6" stroke={palette.accent} strokeWidth="1.75" />
    </svg>
  );
}

export function OGFrame({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        width: "100%",
        height: "100%",
        position: "relative",
        overflow: "hidden",
        background: `linear-gradient(160deg, #0d151e 0%, ${palette.bg} 40%, #101d2a 70%, ${palette.bgCard} 100%)`,
      }}
    >
      <DotGrid />

      {/* Primary warm glow — top right */}
      <div
        style={{
          ...abs,
          top: "-200px",
          right: "-80px",
          width: "700px",
          height: "700px",
          borderRadius: "50%",
          background: `radial-gradient(circle, ${palette.accent}18 0%, ${palette.accent}08 40%, transparent 70%)`,
        }}
      />

      {/* Secondary glow — bottom left */}
      <div
        style={{
          ...abs,
          bottom: "-250px",
          left: "-150px",
          width: "600px",
          height: "600px",
          borderRadius: "50%",
          background: `radial-gradient(circle, ${palette.accent}0c 0%, transparent 65%)`,
        }}
      />

      <div
        style={{
          ...abs,
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          width: "800px",
          height: "400px",
          borderRadius: "50%",
          background: `radial-gradient(ellipse, #1a2d4510 0%, transparent 70%)`,
        }}
      />

      <FunnelWatermark />

      {children}

      <div
        style={{
          ...abs,
          bottom: "0",
          left: "0",
          right: "0",
          height: "3px",
          background: `linear-gradient(90deg, transparent, ${palette.accentDark} 20%, ${palette.accent} 50%, ${palette.accentDark} 80%, transparent)`,
        }}
      />
    </div>
  );
}

function FunnelIcon({ size = 24 }: { size?: number }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      fill="none"
      width={size}
      height={size}
    >
      <path
        d="M8 8h16l-5.5 12h-5L8 8z M22.625 11L27 8"
        stroke={palette.accent}
        strokeWidth="1.75"
        strokeLinejoin="miter"
        fill="none"
      />
      <path d="M16 20v6" stroke={palette.accent} strokeWidth="1.75" />
    </svg>
  );
}

export function OGLogo() {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: "14px" }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          width: "40px",
          height: "40px",
          borderRadius: "10px",
          background: `linear-gradient(135deg, ${palette.accent}15, ${palette.accent}08)`,
          border: `1px solid ${palette.accent}25`,
        }}
      >
        <FunnelIcon size={24} />
      </div>
      <div
        style={{
          display: "flex",
          ...serif,
          fontSize: "22px",
          color: palette.muted,
          letterSpacing: "-0.01em",
        }}
      >
        funnel
      </div>
    </div>
  );
}

export function OGTitle({
  children,
  size = 64,
}: {
  children: ReactNode;
  size?: number;
}) {
  return (
    <div
      style={{
        display: "flex",
        ...serif,
        fontSize: size,
        lineHeight: 1.1,
        letterSpacing: "-0.02em",
        color: palette.text,
        maxWidth: "850px",
      }}
    >
      {children}
    </div>
  );
}

export function OGDescription({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        fontSize: 22,
        color: palette.subtle,
        marginTop: 16,
        lineHeight: 1.5,
        maxWidth: "700px",
      }}
    >
      {children}
    </div>
  );
}
