<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="2.0"
  xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:sitemap="http://www.sitemaps.org/schemas/sitemap/0.9">

  <xsl:output method="html" encoding="UTF-8" indent="yes" />

  <xsl:template match="/">
    <html lang="en">
      <head>
        <meta charset="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>Sitemap - funnel</title>
        <link rel="icon" href="/icon.svg" type="image/svg+xml" />
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="" />
        <link href="https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;500;600&amp;family=DM+Serif+Display&amp;display=swap" rel="stylesheet" />
        <style>
          * { margin: 0; padding: 0; box-sizing: border-box; }

          body {
            font-family: 'DM Sans', system-ui, sans-serif;
            background: oklch(0.975 0.008 70);
            color: oklch(0.18 0.035 230);
            padding: 3rem 1.5rem 4rem;
            max-width: 42rem;
            margin: 0 auto;
            -webkit-font-smoothing: antialiased;
          }

          h1 {
            font-family: 'DM Serif Display', serif;
            font-weight: 400;
            font-size: 2.5rem;
            letter-spacing: -0.02em;
            line-height: 1;
          }

          .meta {
            font-size: 0.75rem;
            color: oklch(0.42 0.025 230);
            margin-top: 0.5rem;
            margin-bottom: 2.5rem;
          }

          table {
            width: 100%;
            border-collapse: collapse;
            font-size: 13px;
          }

          thead th {
            text-align: left;
            font-weight: 600;
            font-size: 13px;
            color: oklch(0.18 0.035 230);
            padding: 0 0 0.625rem;
          }

          thead tr {
            border-bottom: 1px solid oklch(0.895 0.012 70);
          }

          tbody td {
            padding: 0.5rem 0;
            border-bottom: 1px solid oklch(0.895 0.012 70);
          }

          thead th:nth-child(2),
          thead th:nth-child(3),
          thead th:nth-child(4),
          tbody td:nth-child(2),
          tbody td:nth-child(3),
          tbody td:nth-child(4) {
            text-align: right;
          }

          thead th:first-child,
          tbody td:first-child {
            padding-right: 1rem;
          }

          thead th + th,
          tbody td + td {
            padding-left: 1rem;
          }

          a {
            color: oklch(0.18 0.035 230);
            text-decoration: none;
            background-image: linear-gradient(currentColor, currentColor);
            background-size: 0% 1px;
            background-position: left bottom;
            background-repeat: no-repeat;
            transition: background-size 0.25s ease;
          }

          a:hover {
            background-size: 100% 1px;
          }

          .muted {
            font-variant-numeric: tabular-nums;
            color: oklch(0.42 0.025 230);
          }

          .back {
            display: inline-block;
            margin-bottom: 2rem;
            font-size: 13px;
            color: oklch(0.42 0.025 230);
          }

          .back:hover { color: oklch(0.18 0.035 230); }

          * {
            scrollbar-width: thin;
            scrollbar-color: oklch(0.82 0.015 65) transparent;
          }

          ::selection {
            background: oklch(0.55 0.13 65 / 0.2);
          }

          @media (prefers-color-scheme: dark) {
            body {
              background: oklch(0.20 0.045 230);
              color: oklch(0.92 0.012 60);
            }

            thead th {
              color: oklch(0.92 0.012 60);
            }

            thead tr {
              border-bottom-color: oklch(0.30 0.042 230);
            }

            tbody td {
              border-bottom-color: oklch(0.30 0.042 230);
            }

            a {
              color: oklch(0.92 0.012 60);
            }

            .muted {
              color: oklch(0.62 0.025 230);
            }

            .back {
              color: oklch(0.62 0.025 230);
            }

            .back:hover { color: oklch(0.92 0.012 60); }

            * {
              scrollbar-color: oklch(0.32 0.04 230) transparent;
            }

            ::selection {
              background: oklch(0.78 0.12 65 / 0.2);
            }
          }
        </style>
      </head>
      <body>
        <a class="back" href="/">&#8592; funnel.karolbroda.com</a>
        <h1>Sitemap</h1>
        <p class="meta">
          <xsl:value-of select="count(sitemap:urlset/sitemap:url)" /> URLs
        </p>
        <table>
          <thead>
            <tr>
              <th>URL</th>
              <th>Priority</th>
              <th>Frequency</th>
              <th>Modified</th>
            </tr>
          </thead>
          <tbody>
            <xsl:for-each select="sitemap:urlset/sitemap:url">
              <xsl:sort select="sitemap:priority" order="descending" data-type="number" />
              <tr>
                <td>
                  <a href="{sitemap:loc}"><xsl:value-of select="sitemap:loc" /></a>
                </td>
                <td class="muted">
                  <xsl:value-of select="sitemap:priority" />
                </td>
                <td class="muted">
                  <xsl:value-of select="sitemap:changefreq" />
                </td>
                <td class="muted">
                  <xsl:value-of select="substring(sitemap:lastmod, 1, 10)" />
                </td>
              </tr>
            </xsl:for-each>
          </tbody>
        </table>
      </body>
    </html>
  </xsl:template>
</xsl:stylesheet>
