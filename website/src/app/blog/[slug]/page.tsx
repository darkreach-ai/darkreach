import { notFound } from "next/navigation";
import { blogPosts } from "@/lib/blog-posts";
import { BlogPostLayout } from "@/components/blog-post-layout";
import type { Metadata } from "next";

const mdxModules: Record<string, () => Promise<{ default: React.ComponentType }>> = {
  "announcing-darkreach": () => import("@content/blog/announcing-darkreach.mdx"),
  "prime-discovery-initiative": () => import("@content/blog/prime-discovery-initiative.mdx"),
  "why-open-source": () => import("@content/blog/why-open-source.mdx"),
};

export function generateStaticParams() {
  return blogPosts.map((post) => ({ slug: post.slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ slug: string }>;
}): Promise<Metadata> {
  const { slug } = await params;
  const post = blogPosts.find((p) => p.slug === slug);
  if (!post) return {};

  return {
    title: post.title,
    description: post.excerpt,
    openGraph: {
      title: post.title,
      description: post.excerpt,
      type: "article",
      publishedTime: post.date,
    },
  };
}

export default async function BlogPostPage({
  params,
}: {
  params: Promise<{ slug: string }>;
}) {
  const { slug } = await params;
  const post = blogPosts.find((p) => p.slug === slug);

  if (!post || !mdxModules[slug]) {
    notFound();
  }

  const { default: MDXContent } = await mdxModules[slug]();

  return (
    <BlogPostLayout post={post}>
      <MDXContent />
    </BlogPostLayout>
  );
}
