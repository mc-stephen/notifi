import { DocsLayout } from 'fumadocs-ui/layouts/docs';
import { source } from '@/lib/source';
import { baseOptions } from '@/lib/layout.config';
import type { ReactNode } from 'react';

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <DocsLayout
      {...baseOptions()}
      tree={source.getPageTree()}
      tabs={[
        {
          title: 'Docs',
          description: 'Guides, concepts, and references',
          url: '/docs',
        },
        {
          title: 'API Reference',
          description: 'REST API endpoints and SDKs',
          url: '/docs/api-reference',
        },
        {
          title: 'Security',
          description: 'Authentication, encryption, and compliance',
          url: '/docs/security',
        },
      ]}
    >
      {children}
    </DocsLayout>
  );
}
