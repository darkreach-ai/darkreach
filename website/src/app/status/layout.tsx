import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "System Status",
  description: "Live service health and network overview for darkreach.",
};

export default function StatusLayout({ children }: { children: React.ReactNode }) {
  return children;
}
