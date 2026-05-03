export const metadata = {
  title: "ChimiaClaw Three-Agent Pipeline",
  description: "Live world-map dashboard for the ChimiaClaw multi-agent science pipeline",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
