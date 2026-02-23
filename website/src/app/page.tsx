import { Hero } from "@/components/hero";
import { ProofBar } from "@/components/proof-bar";
import { VisionCards } from "@/components/vision-cards";
import { HowItWorks } from "@/components/how-it-works";
import { InitiativePreview } from "@/components/initiative-preview";
import { OperatorRecruit } from "@/components/operator-recruit";
import { SocialProof } from "@/components/social-proof";
import { WaitlistCTA } from "@/components/waitlist-cta";

function Divider() {
  return <div className="section-divider" />;
}

export default function Home() {
  return (
    <>
      <Hero />
      <ProofBar />
      <Divider />
      <VisionCards />
      <HowItWorks />
      <Divider />
      <InitiativePreview />
      <OperatorRecruit />
      <Divider />
      <SocialProof />
      <WaitlistCTA />
    </>
  );
}
