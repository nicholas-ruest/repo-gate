import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'RepoGate',
  description: 'Deep repository assessment for open-core gating',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
