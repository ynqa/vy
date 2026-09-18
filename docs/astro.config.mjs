import { defineConfig } from 'astro/config';
import { unified } from '@astrojs/markdown-remark';
import starlight from '@astrojs/starlight';
import remarkDocs from './src/plugins/remark-docs.mjs';

const base = `/${(process.env.BASE_PATH || '').replace(/^\/+|\/+$/g, '')}`.replace(/\/?$/, '/');

export default defineConfig({
  site: process.env.SITE_URL || 'https://vy.ynqa.dev',
  base,
  trailingSlash: 'ignore',
  output: 'static',
  markdown: {
    processor: unified({ remarkPlugins: [[remarkDocs, { base }]] }),
  },
  integrations: [
    starlight({
      title: 'vy',
      logo: { src: './public/logo.png', alt: 'vy', replacesTitle: true },
      favicon: '/favicon.png',
      description: 'Explore and query JSON and YAML, right in your terminal.',
      locales: { root: { label: 'English', lang: 'en' } },
      customCss: ['./src/styles/theme.css'],
      components: {
        ThemeProvider: './src/components/ThemeProvider.astro',
        ThemeSelect: './src/components/ThemeSelect.astro',
        MarkdownContent: './src/components/MarkdownContent.astro',
      },
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/ynqa/vy' }],
      editLink: { baseUrl: 'https://github.com/ynqa/vy/edit/main/docs/' },
      credits: false,
      expressiveCode: {
        themes: ['github-dark'],
        styleOverrides: {
          codeBackground: '#191B1F',
          borderColor: '#30343B',
          codeFontFamily: '"IBM Plex Mono", "IBM Plex Sans JP", monospace',
        },
      },
      head: [
        { tag: 'meta', attrs: { name: 'theme-color', content: '#101113' } },
        { tag: 'link', attrs: { rel: 'preconnect', href: 'https://fonts.googleapis.com' } },
        { tag: 'link', attrs: { rel: 'preconnect', href: 'https://fonts.gstatic.com', crossorigin: 'anonymous' } },
        {
          tag: 'link',
          attrs: {
            rel: 'stylesheet',
            href: 'https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&family=IBM+Plex+Sans+JP:wght@400;500;600&display=swap',
          },
        },
      ],
      sidebar: [
        { label: 'Overview', slug: '' },
        { label: 'Getting started', slug: 'getting-started' },
        {
          label: 'Explore',
          items: [
            { label: 'Navigation', slug: 'explore/navigation' },
            { label: 'Search', slug: 'explore/search' },
            { label: 'Combining focus and jaq', slug: 'explore/focus-and-jaq' },
            { label: 'Commands', items: [{ autogenerate: { directory: 'explore/commands' } }] },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'Commands', slug: 'reference/commands' },
            { label: 'Settings', items: [{ autogenerate: { directory: 'reference/settings' } }] },
            { label: 'vs fx, jless', slug: 'reference/comparison' },
          ],
        },
        { label: 'Changelog', slug: 'changelog' },
        { label: 'Development', items: [{ autogenerate: { directory: 'development' } }] },
      ],
    }),
  ],
});
