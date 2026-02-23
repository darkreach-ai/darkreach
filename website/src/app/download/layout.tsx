import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Download",
  description: "Download and install the darkreach node software.",
};

export default function DownloadLayout({ children }: { children: React.ReactNode }) {
  return children;
}
