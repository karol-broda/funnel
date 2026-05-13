import { palette } from "@/lib/colors";
import { createOGResponse, OGFrame } from "@/lib/og";

export const revalidate = 3600;

export function GET() {
  return createOGResponse(
    <OGFrame>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          width: "100%",
          height: "100%",
          position: "relative",
        }}
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          viewBox="0 0 32 32"
          fill="none"
          width="56"
          height="56"
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

        <div
          style={{
            display: "flex",
            fontFamily: "DM Serif Display",
            fontSize: 120,
            letterSpacing: "-0.03em",
            color: palette.text,
            marginTop: 16,
          }}
        >
          funnel
        </div>

        <div
          style={{
            display: "flex",
            width: "48px",
            height: "2px",
            background: `linear-gradient(90deg, transparent, ${palette.accent}, transparent)`,
            marginTop: 20,
          }}
        />

        <div
          style={{
            display: "flex",
            fontSize: 20,
            color: palette.muted,
            marginTop: 20,
            letterSpacing: "0.2em",
            textTransform: "uppercase" as const,
          }}
        >
          Self-hosted tunnels over QUIC
        </div>
      </div>
    </OGFrame>,
  );
}
