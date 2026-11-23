import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "rust-next-voiler",
  description: "Rust + Next.js + MySQL boilerplate",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="ja">
      <body>{children}</body>
    </html>
  );
}
