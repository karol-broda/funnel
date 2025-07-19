import { ImageResponse } from "next/og";
import { NextRequest } from "next/server";
import { flavors } from "@catppuccin/palette";

const macchiato = flavors.macchiato;

export const runtime = "edge";

export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const title =
      searchParams.get("title") || "localhost wants to see other people";
    const description =
      searchParams.get("description") ||
      "yet another tunneling tool nobody asked for";

    return new ImageResponse(
      (
        <div
          style={{
            height: "100%",
            width: "100%",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            background: `linear-gradient(135deg, ${macchiato.colors.crust.hex} 0%, #1e1e2e 50%, ${macchiato.colors.crust.hex} 100%)`,
            position: "relative",
            padding: "60px",
          }}
        >
          <div
            style={{
              position: "absolute",
              top: "50px",
              right: "50px",
              width: "60px",
              height: "60px",
              backgroundColor: macchiato.colors.mauve.hex,
              opacity: 0.3,
              borderRadius: "8px",
            }}
          />
          <div
            style={{
              position: "absolute",
              bottom: "80px",
              left: "80px",
              width: "40px",
              height: "40px",
              backgroundColor: macchiato.colors.green.hex,
              opacity: 0.4,
              borderRadius: "50%",
            }}
          />
          <div
            style={{
              display: "flex",
              alignItems: "center",
              marginBottom: "30px",
              backgroundColor: `${macchiato.colors.surface0.hex}40`,
              borderRadius: "25px",
              padding: "12px 24px",
              border: `1px solid ${macchiato.colors.surface1.hex}40`,
            }}
          >
            <span style={{ marginRight: "12px", fontSize: "20px" }}>✨</span>
            <span
              style={{
                color: macchiato.colors.subtext1.hex,
                fontSize: "18px",
              }}
            >
              {description}
            </span>
          </div>
          <h1
            style={{
              fontSize: "72px",
              fontWeight: "bold",
              background: `linear-gradient(135deg, ${macchiato.colors.mauve.hex}, ${macchiato.colors.pink.hex})`,
              backgroundClip: "text",
              color: "transparent",
              lineHeight: "1.1",
              margin: "0 0 40px 0",
              textAlign: "center",
              maxWidth: "1000px",
            }}
          >
            {title}
          </h1>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              color: macchiato.colors.green.hex,
              fontSize: "24px",
              fontWeight: "600",
              backgroundColor: `${macchiato.colors.base.hex}80`,
              borderRadius: "16px",
              padding: "16px 32px",
              border: `1px solid ${macchiato.colors.surface0.hex}`,
            }}
          >
            <span style={{ marginRight: "12px" }}>$</span>
            <span
              style={{ color: macchiato.colors.blue.hex, marginRight: "12px" }}
            >
              funnel
            </span>
            <span
              style={{
                color: macchiato.colors.yellow.hex,
                marginRight: "12px",
              }}
            >
              http
            </span>
            <span style={{ color: macchiato.colors.text.hex }}>3000</span>
          </div>
        </div>
      ),
      {
        width: 1200,
        height: 630,
      }
    );
  } catch (error) {
    console.error("Error generating OG image:", error);
    return new Response("Failed to generate image", { status: 500 });
  }
}
