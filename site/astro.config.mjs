// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// Served from GitHub Pages as a project site:
// https://zhuowcui.github.io/open-ui/
export default defineConfig({
  site: 'https://zhuowcui.github.io',
  base: '/open-ui',
  trailingSlash: 'ignore',
  integrations: [
    starlight({
      title: 'Open UI',
      description:
        'An experimental native Rust UI framework that draws CSS layout and paint with Skia, measured pixel by pixel against Chromium.',
      favicon: '/favicon.svg',
      customCss: ['./src/styles/fonts.css', './src/styles/theme.css'],
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/zhuowcui/open-ui' },
      ],
      editLink: {
        baseUrl: 'https://github.com/zhuowcui/open-ui/edit/main/site/',
      },
      head: [
        { tag: 'meta', attrs: { name: 'color-scheme', content: 'light dark' } },
      ],
      sidebar: [
        { label: 'Getting started', link: '/getting-started/' },
        { label: 'Fidelity', link: '/fidelity/' },
        { label: 'Docs', link: '/docs/' },
        { label: 'Components', link: '/components/' },
        { label: 'Roadmap', link: '/roadmap/' },
        { label: 'Devlog', link: '/devlog/' },
        { label: 'About this site', link: '/about/' },
      ],
    }),
  ],
});
