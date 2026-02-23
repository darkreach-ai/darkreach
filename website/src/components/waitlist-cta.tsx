import { Section } from "./ui/section";
import { ScrollAnimate } from "./scroll-animate";
import { WaitlistForm } from "./waitlist-form";

export function WaitlistCTA() {
  return (
    <Section className="cta-gradient">
      <ScrollAnimate>
        <div className="text-center max-w-2xl mx-auto">
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            Be part of something bigger
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-8">
            Join researchers and operators building the world&apos;s biggest
            supercomputer. Get early access when we launch.
          </p>
          <div className="flex justify-center">
            <WaitlistForm variant="full" />
          </div>
        </div>
      </ScrollAnimate>
    </Section>
  );
}
