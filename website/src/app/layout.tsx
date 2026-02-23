import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { Navbar } from "@/components/navbar";
import { Footer } from "@/components/footer";
import { DiscoveryToast } from "@/components/discovery-toast";
import "./globals.css";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

export const metadata: Metadata = {
  title: {
    default: "darkreach — The World's Biggest Supercomputer",
    template: "%s — darkreach",
  },
  description:
    "Building the world's biggest supercomputer. AI-orchestrated distributed computing for scientific discovery.",
  icons: {
    icon: "/favicon.svg",
  },
  openGraph: {
    title: "darkreach — The World's Biggest Supercomputer",
    description:
      "Building the world's biggest supercomputer. AI-orchestrated distributed computing for scientific discovery.",
    url: "https://darkreach.ai",
    siteName: "darkreach",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: "darkreach — The World's Biggest Supercomputer",
    description:
      "Building the world's biggest supercomputer. AI-orchestrated distributed computing for scientific discovery.",
  },
};

const jsonLd = {
  "@context": "https://schema.org",
  "@graph": [
    {
      "@type": "Organization",
      name: "darkreach",
      url: "https://darkreach.ai",
      logo: "https://darkreach.ai/favicon.svg",
      sameAs: [
        "https://github.com/darkreach-ai/darkreach",
      ],
      description:
        "Building the world's biggest supercomputer. AI-orchestrated distributed computing for scientific discovery.",
    },
    {
      "@type": "WebSite",
      name: "darkreach",
      url: "https://darkreach.ai",
      description:
        "AI-orchestrated distributed computing platform for prime number discovery and scientific research.",
    },
  ],
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={inter.variable}>
      <head>
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
      </head>
      <body className="min-h-screen bg-background text-foreground antialiased">
        <Navbar />
        <main className="pt-16">{children}</main>
        <Footer />
        <DiscoveryToast />
      </body>
    </html>
  );
}
