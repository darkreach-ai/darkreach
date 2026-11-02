import { Section } from "@/components/ui/section";
import { BlogCard } from "@/components/blog-card";
import { getAllPosts } from "@/lib/blog-posts";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Blog",
  description: "News, updates, and deep dives from the darkreach team.",
};

export default function BlogPage() {
  const posts = getAllPosts();

  return (
    <Section>
      <h1 className="text-4xl font-bold text-foreground mb-4">Blog</h1>
      <p className="text-muted-foreground mb-10">
        News, updates, and deep dives from the darkreach team.
      </p>

      <div className="space-y-6">
        {posts.map((post) => (
          <BlogCard key={post.slug} post={post} />
        ))}
      </div>
    </Section>
  );
}
