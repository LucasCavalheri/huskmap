import type { APIRoute } from "astro";

// Both languages, each pointing at the other with hreflang.
export const GET: APIRoute = ({ site }) => {
  const en = new URL("/", site).href;
  const pt = new URL("/pt-br/", site).href;
  const alt = `<xhtml:link rel="alternate" hreflang="en" href="${en}"/><xhtml:link rel="alternate" hreflang="pt-BR" href="${pt}"/>`;
  const body = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <url><loc>${en}</loc>${alt}</url>
  <url><loc>${pt}</loc>${alt}</url>
</urlset>
`;
  return new Response(body, { headers: { "Content-Type": "application/xml" } });
};
