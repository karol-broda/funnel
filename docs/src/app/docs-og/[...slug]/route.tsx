import { ImageResponse } from "next/og";
import { source } from "@/lib/source";
import { notFound } from "next/navigation";
import { flavors } from "@catppuccin/palette";

const macchiato = flavors.macchiato;

export async function GET(
  _req: Request,
  { params }: { params: Promise<{ slug: string[] }> }
) {
  const { slug } = await params;
  const pageName = slug.slice(0, -1);
  console.log({ pageName });

  const page = source.getPage(pageName);
  if (!page) {
    notFound();
  }

  return new ImageResponse(
    (
      <div
        style={{
          height: "100%",
          width: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          backgroundColor: macchiato.colors.crust.hex,
          position: "relative",
        }}
      >
        <div
          style={{
            position: "absolute",
            top: "-60px",
            left: "-60px",
            width: "200px",
            height: "200px",
            backgroundColor: macchiato.colors.green.hex,
            opacity: 0.15,
            borderRadius: "8px",
          }}
        />
        <div
          style={{
            position: "absolute",
            top: "80px",
            right: "120px",
            width: "100px",
            height: "100px",
            backgroundColor: macchiato.colors.blue.hex,
            opacity: 0.2,
            borderRadius: "4px",
          }}
        />
        <div
          style={{
            position: "absolute",
            bottom: "120px",
            left: "100px",
            width: "120px",
            height: "120px",
            backgroundColor: macchiato.colors.yellow.hex,
            opacity: 0.18,
            borderRadius: "6px",
          }}
        />
        <div
          style={{
            position: "absolute",
            bottom: "100px",
            right: "80px",
            width: "90px",
            height: "60px",
            backgroundColor: macchiato.colors.red.hex,
            opacity: 0.2,
            borderRadius: "4px",
          }}
        />
        <div
          style={{
            position: "absolute",
            top: "180px",
            left: "180px",
            width: "12px",
            height: "12px",
            backgroundColor: macchiato.colors.green.hex,
            opacity: 0.8,
          }}
        />

        <div
          style={{
            position: "absolute",
            top: "320px",
            right: "250px",
            width: "8px",
            height: "8px",
            backgroundColor: macchiato.colors.blue.hex,
            opacity: 0.9,
          }}
        />

        <div
          style={{
            position: "absolute",
            bottom: "200px",
            left: "300px",
            width: "10px",
            height: "10px",
            backgroundColor: macchiato.colors.yellow.hex,
            opacity: 0.8,
          }}
        />
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            width: "800px",
            backgroundColor: macchiato.colors.base.hex,
            borderRadius: "12px",
            border: `2px solid ${macchiato.colors.surface0.hex}`,
            overflow: "hidden",
            zIndex: 10,
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              padding: "16px 20px",
              backgroundColor: macchiato.colors.mantle.hex,
              borderBottom: `1px solid ${macchiato.colors.surface0.hex}`,
            }}
          >
            <div
              style={{
                display: "flex",
                gap: "8px",
                marginRight: "20px",
              }}
            >
              <div
                style={{
                  width: "12px",
                  height: "12px",
                  borderRadius: "50%",
                  backgroundColor: macchiato.colors.red.hex,
                }}
              />
              <div
                style={{
                  width: "12px",
                  height: "12px",
                  borderRadius: "50%",
                  backgroundColor: macchiato.colors.yellow.hex,
                }}
              />
              <div
                style={{
                  width: "12px",
                  height: "12px",
                  borderRadius: "50%",
                  backgroundColor: macchiato.colors.green.hex,
                }}
              />
            </div>
            <div
              style={{
                color: macchiato.colors.subtext1.hex,
                fontSize: "16px",
                fontWeight: "normal",
              }}
            >
              localhost:3000 → funnel.karolbroda.com
            </div>
          </div>
          <div
            style={{
              padding: "50px 40px",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              textAlign: "center",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                marginBottom: "35px",
                color: macchiato.colors.green.hex,
                fontSize: "20px",
                fontWeight: "bold",
              }}
            >
              <span style={{ marginRight: "10px" }}>$</span>
              <span
                style={{ color: macchiato.colors.blue.hex, marginRight: "8px" }}
              >
                funnel
              </span>
              <span
                style={{
                  color: macchiato.colors.yellow.hex,
                  marginRight: "8px",
                }}
              >
                http
              </span>
              <span style={{ color: macchiato.colors.text.hex }}>3000</span>
            </div>
            <h1
              style={{
                fontSize: "58px",
                fontWeight: "bold",
                color: macchiato.colors.mauve.hex,
                lineHeight: "1.1",
                margin: "0 0 25px 0",
                textAlign: "center",
              }}
            >
              {page.data.title}
            </h1>
            {page.data.description && (
              <p
                style={{
                  fontSize: "24px",
                  color: macchiato.colors.text.hex,
                  lineHeight: "1.4",
                  margin: "0 0 30px 0",
                  maxWidth: "650px",
                }}
              >
                {page.data.description}
              </p>
            )}
          </div>
        </div>
      </div>
    ),
    {
      width: 1200,
      height: 630,
    }
  );
}

export function generateStaticParams() {
  const params = source.generateParams().map((page) => ({
    ...page,
    slug: [...page.slug, "image.png"],
  }));
  console.log(params);
  return params;
}
