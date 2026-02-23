import { Section } from "./ui/section";

interface PageHeroProps {
  title: string;
  description: string;
  eyebrow?: string;
}

export function PageHero({ title, description, eyebrow }: PageHeroProps) {
  return (
    <Section className="py-24">
      <div className="text-center max-w-3xl mx-auto">
        {eyebrow && (
          <p className="text-sm uppercase tracking-wider text-accent-purple font-medium mb-3">
            {eyebrow}
          </p>
        )}
        <h1 className="text-4xl sm:text-5xl font-bold tracking-[-0.03em] text-foreground mb-6">
          {title}
        </h1>
        <p className="text-lg text-muted-foreground max-w-2xl mx-auto leading-relaxed">
          {description}
        </p>
      </div>
    </Section>
  );
}
