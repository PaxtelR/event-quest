import type { Metadata } from "next";
import { Geist_Mono, Inter } from "next/font/google";

import { DevnetBanner } from "@/components/DevnetBanner";
import { SiteHeader } from "@/components/SiteHeader";
import { messages } from "@/i18n/en-US";

import { Providers } from "./providers";
import "./globals.css";

const inter = Inter({ variable: "--font-sans", subsets: ["latin"] });
const geistMono = Geist_Mono({ variable: "--font-mono", subsets: ["latin"] });

export const metadata: Metadata = {
  title: `${messages.common.appName} — ${messages.home.tagline}`,
  description: messages.home.description,
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${inter.variable} ${geistMono.variable}`}>
      <body>
        <Providers>
          <div className="flex min-h-screen flex-col">
            <DevnetBanner />
            <SiteHeader />
            {children}
          </div>
        </Providers>
      </body>
    </html>
  );
}
