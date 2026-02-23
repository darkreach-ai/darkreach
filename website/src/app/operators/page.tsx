"use client";

import { PageHero } from "@/components/page-hero";
import { Section } from "@/components/ui/section";
import { Card } from "@/components/ui/card";
import { InstallCommand } from "@/components/install-command";
import { NetworkStats } from "@/components/network-stats";
import { ScrollAnimate } from "@/components/scroll-animate";
import { systemRequirements } from "@/lib/install-commands";
import { Trophy, Award, Code, Server, Monitor, ArrowRight } from "lucide-react";
import Link from "next/link";

const reasons = [
  {
    icon: Trophy,
    title: "Permanent Impact",
    description:
      "Every discovery is permanently credited to you. Your name goes on the result, the certificate, and the leaderboard.",
    accent: "from-amber-500 to-orange-500",
  },
  {
    icon: Award,
    title: "Earn Recognition",
    description:
      "Climb the leaderboard rankings. Top operators get featured on the website and earn badges for milestones.",
    accent: "from-indigo-500 to-violet-500",
  },
  {
    icon: Code,
    title: "Open Source",
    description:
      "MIT license, full transparency. Read the code, understand the algorithms, contribute improvements.",
    accent: "from-emerald-500 to-teal-500",
  },
];

export default function OperatorsPage() {
  return (
    <>
      <PageHero
        eyebrow="Operators"
        title="Your machine. Global discoveries."
        description="Contribute spare compute to scientific research. Your machine runs the algorithms, the network verifies the results, and you get credited for every discovery."
      />

      {/* Why Contribute */}
      <Section secondary>
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            Why contribute
          </h2>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {reasons.map((reason, i) => (
            <ScrollAnimate key={reason.title} delay={i * 100}>
              <Card hover className="h-full">
                <div
                  className={`w-10 h-10 rounded-lg bg-gradient-to-br ${reason.accent} flex items-center justify-center text-white mb-4`}
                >
                  <reason.icon size={20} />
                </div>
                <h3 className="text-foreground font-semibold mb-2">
                  {reason.title}
                </h3>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {reason.description}
                </p>
              </Card>
            </ScrollAnimate>
          ))}
        </div>
      </Section>

      {/* Get Started */}
      <Section>
        <div className="text-center mb-12">
          <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
            Installation
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            Get started in 3 steps
          </h2>
          <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
            Install darkreach, register as an operator, and start contributing
            compute. Detected your OS automatically.
          </p>
        </div>

        <div className="max-w-3xl mx-auto">
          <InstallCommand />
        </div>
      </Section>

      {/* System Requirements */}
      <Section secondary>
        <h2 className="text-2xl font-bold text-foreground mb-8">
          System Requirements
        </h2>

        <div className="overflow-x-auto rounded-lg border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="bg-background text-muted-foreground text-left">
                <th className="px-4 py-3 font-medium">Component</th>
                <th className="px-4 py-3 font-medium">Minimum</th>
                <th className="px-4 py-3 font-medium">Recommended</th>
              </tr>
            </thead>
            <tbody>
              {systemRequirements.map((req) => (
                <tr key={req.component} className="border-t border-border">
                  <td className="px-4 py-3 text-foreground font-medium">
                    {req.component}
                  </td>
                  <td className="px-4 py-3 text-muted-foreground">
                    {req.minimum}
                  </td>
                  <td className="px-4 py-3 text-muted-foreground">
                    {req.recommended}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Section>

      {/* Deployment Guides */}
      <Section>
        <h2 className="text-2xl font-bold text-foreground mb-8">
          Deployment Guides
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <Link href="/download/server">
            <Card hover className="group cursor-pointer">
              <div className="flex items-center gap-3 mb-3">
                <div className="w-10 h-10 rounded-lg bg-accent-purple/10 border border-accent-purple/30 flex items-center justify-center text-accent-purple">
                  <Server size={20} />
                </div>
                <h3 className="text-lg font-semibold text-foreground">
                  Coordinator Setup
                </h3>
                <ArrowRight
                  size={16}
                  className="ml-auto text-muted-foreground group-hover:text-accent-purple transition-colors"
                />
              </div>
              <p className="text-sm text-muted-foreground">
                Deploy a self-hosted coordinator with PostgreSQL, systemd
                services, and the real-time dashboard.
              </p>
            </Card>
          </Link>

          <Link href="/download/worker">
            <Card hover className="group cursor-pointer">
              <div className="flex items-center gap-3 mb-3">
                <div className="w-10 h-10 rounded-lg bg-accent-green/10 border border-accent-green/30 flex items-center justify-center text-accent-green">
                  <Monitor size={20} />
                </div>
                <h3 className="text-lg font-semibold text-foreground">
                  Worker Deployment
                </h3>
                <ArrowRight
                  size={16}
                  className="ml-auto text-muted-foreground group-hover:text-accent-green transition-colors"
                />
              </div>
              <p className="text-sm text-muted-foreground">
                Connect worker nodes to a coordinator and contribute compute to
                the prime search network.
              </p>
            </Card>
          </Link>
        </div>
      </Section>

      {/* Network Stats */}
      <Section secondary>
        <div className="text-center mb-8">
          <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
            Network
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            The network right now
          </h2>
        </div>
        <div className="max-w-2xl mx-auto">
          <NetworkStats />
        </div>
      </Section>

      {/* CTA */}
      <Section>
        <ScrollAnimate>
          <div className="text-center">
            <h2 className="text-2xl font-bold text-foreground mb-4">
              Join the network
            </h2>
            <p className="text-muted-foreground mb-6">
              See how operators rank on the leaderboard.
            </p>
            <a
              href="/leaderboard"
              className="inline-flex items-center gap-2 px-6 py-3 rounded-md bg-accent-purple text-white font-medium hover:bg-accent-purple/90 transition-colors"
            >
              View leaderboard
              <ArrowRight size={16} />
            </a>
          </div>
        </ScrollAnimate>
      </Section>
    </>
  );
}
