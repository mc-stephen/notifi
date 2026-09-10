import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span className="flex items-center gap-2.5 font-semibold text-base">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="text-fd-primary">
            <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
          </svg>
          Notifi
        </span>
      ),
    },
    githubUrl: 'https://github.com/anomalyco/notifi',
  };
}
