import { RootProvider } from 'fumadocs-ui/provider/next';
import './globals.css';
import type { ReactNode } from 'react';
import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: {
    template: '%s | Notifi Docs',
    default: 'Notifi Docs',
  },
  description: 'Developer documentation for the Notifi notification platform',
};

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="flex min-h-screen flex-col">
        <RootProvider
          search={{
            options: {
              type: 'static',
              api: '/search-index.json',
            },
          }}
        >
          {children}
        </RootProvider>
      </body>
    </html>
  );
}
