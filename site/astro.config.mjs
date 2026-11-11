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
        'Native desktop apps in Rust, styled with the CSS you know. No webview, one binary.',
      favicon: '/favicon.svg',
      logo: { src: './src/assets/logo.svg' },
      customCss: ['./src/styles/fonts.css', './src/styles/theme.css'],
      components: {
        SocialIcons: './src/components/HeaderLinks.astro',
      },
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
        {
          label: 'Start here',
          items: [
            { label: 'Introduction', link: '/docs/' },
            { label: 'Getting started', link: '/docs/getting-started/' },
          ],
        },
        {
          label: 'Guides',
          items: [
            { label: 'Views and styles', link: '/docs/views-and-styles/' },
            { label: 'Testing', link: '/docs/testing/' },
            { label: 'Rendering', link: '/docs/rendering/' },
          ],
        },
        { label: 'Components', link: '/components/' },
        {
          label: 'Project',
          items: [
            { label: 'Roadmap', link: '/docs/roadmap/' },
            { label: 'Blog', link: '/blog/' },
            { label: 'About', link: '/docs/about/' },
          ],
        },
      ],
    }),
  ],
});
