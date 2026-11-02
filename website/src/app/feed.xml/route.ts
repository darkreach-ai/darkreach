import { getAllPosts } from "@/lib/blog-posts";

/** Required for Next.js static export — pre-renders this route at build time. */
export const dynamic = "force-static";

const SITE_URL = "https://darkreach.ai";

/**
 * RSS 2.0 feed generated at build time from MDX blog posts.
 * Accessible at /feed.xml — pre-rendered during static export.
 */
export async function GET() {
  const posts = getAllPosts();

  const items = posts
    .map((post) => {
      const pubDate = new Date(post.date + "T00:00:00Z").toUTCString();
      const escapedTitle = escapeXml(post.title);
      const escapedExcerpt = escapeXml(post.excerpt);

      return `    <item>
      <title>${escapedTitle}</title>
      <link>${SITE_URL}/blog/${post.slug}/</link>
      <description>${escapedExcerpt}</description>
      <pubDate>${pubDate}</pubDate>
      <guid isPermaLink="true">${SITE_URL}/blog/${post.slug}/</guid>
      <author>${escapeXml(post.author)}</author>
    </item>`;
    })
    .join("\n");

  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>darkreach Blog</title>
    <link>${SITE_URL}/blog/</link>
    <description>News, updates, and deep dives from the darkreach team — distributed scientific computing and prime number discovery.</description>
    <language>en-us</language>
    <atom:link href="${SITE_URL}/feed.xml/" rel="self" type="application/rss+xml" />
${items}
  </channel>
</rss>`;

  return new Response(xml, {
    headers: {
      "Content-Type": "application/rss+xml; charset=utf-8",
    },
  });
}

/** Escape special XML characters to produce valid RSS. */
function escapeXml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
