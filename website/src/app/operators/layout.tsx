import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Become an Operator",
  description: "Contribute your compute to scientific discovery. Install darkreach and join the global network.",
};

export default function OperatorsLayout({ children }: { children: React.ReactNode }) {
  return children;
}
