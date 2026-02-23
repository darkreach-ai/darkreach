import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { Badge } from "./ui/badge";
import { Card } from "./ui/card";
import type { BlogPost } from "@/lib/blog-posts";

export function BlogCard({ post }: { post: BlogPost }) {
  return (
    <Link href={`/blog/${post.slug}`}>
      <Card hover className="group">
        <div className="flex items-center gap-2 text-xs text-muted-foreground mb-3">
          <time>{post.date}</time>
          <span>·</span>
          <span>{post.author}</span>
          {post.readingTime && (
            <>
              <span>·</span>
              <span>{post.readingTime}</span>
            </>
          )}
        </div>
        <h2 className="text-lg font-semibold text-foreground mb-2 group-hover:text-accent-purple transition-colors">
          {post.title}
        </h2>
        <p className="text-sm text-muted-foreground leading-relaxed mb-4">
          {post.excerpt}
        </p>
        <div className="flex items-center justify-between">
          <div className="flex flex-wrap gap-2">
            {post.tags.map((tag) => (
              <Badge key={tag}>{tag}</Badge>
            ))}
          </div>
          <ArrowRight className="w-4 h-4 text-muted-foreground group-hover:text-accent-purple transition-colors shrink-0" />
        </div>
      </Card>
    </Link>
  );
}
