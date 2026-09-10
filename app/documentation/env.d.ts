/// <reference types="fumadocs-mdx/types" />

declare module 'fumadocs-mdx:collections/server' {
  export const docs: {
    docs: Record<string, unknown>[];
    meta: Record<string, unknown>[];
    toFumadocsSource(): import('fumadocs-core/source').Source;
  };
  export const meta: {
    docs: Record<string, unknown>[];
    meta: Record<string, unknown>[];
    toFumadocsSource(): import('fumadocs-core/source').Source;
  };
}
