// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { name, repo, siteUrl } from './src/lib/project.mjs';

// Served from GitHub Pages at the domain root, https://div.build/
// (public/CNAME names the domain).
export default defineConfig({
  site: siteUrl,
  base: '/',
  trailingSlash: 'ignore',
  integrations: [
    starlight({
      title: name,
      description:
        'Native desktop apps in Rust, written with the CSS you already know and shipped as a single native binary. No webview, no JavaScript.',
      // The favicons, wordmarks and Open Graph image are copied from brand/
      // by scripts/sync-assets.mjs (hash-checked against scripts/assets.json).
      favicon: '/favicon.svg',
      logo: {
        light: './src/assets/generated/brand/div-wordmark.svg',
        dark: './src/assets/generated/brand/div-wordmark-dark.svg',
        alt: name,
        replacesTitle: true,
      },
      customCss: ['./src/styles/fonts.css', './src/styles/theme.css'],
      components: {
        SocialIcons: './src/components/HeaderLinks.astro',
      },
      social: [
        { icon: 'github', label: 'GitHub', href: repo },
      ],
      editLink: {
        baseUrl: `${repo}/edit/main/site/`,
      },
      head: [
        { tag: 'meta', attrs: { name: 'color-scheme', content: 'light dark' } },
        { tag: 'link', attrs: { rel: 'icon', href: '/favicon.ico', sizes: '32x32' } },
        { tag: 'link', attrs: { rel: 'apple-touch-icon', href: '/apple-touch-icon.png' } },
        { tag: 'link', attrs: { rel: 'manifest', href: '/site.webmanifest' } },
        { tag: 'meta', attrs: { property: 'og:image', content: `${siteUrl}/og-image.png` } },
        { tag: 'meta', attrs: { property: 'og:image:width', content: '1200' } },
        { tag: 'meta', attrs: { property: 'og:image:height', content: '630' } },
        { tag: 'meta', attrs: { property: 'og:image:alt', content: 'The Div logo' } },
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
          label: 'Core concepts',
          items: [
            { label: 'Views and view!', link: '/docs/concepts/views/' },
            { label: 'Elements', link: '/docs/concepts/elements/' },
            { label: 'Styling', link: '/docs/concepts/styling/' },
            { label: 'Layout', link: '/docs/concepts/layout/' },
            { label: 'Text and fonts', link: '/docs/concepts/text-and-fonts/' },
            { label: 'Events and state', link: '/docs/concepts/events-and-state/' },
            { label: 'Apps and windows', link: '/docs/concepts/windows/' },
          ],
        },
        {
          label: 'Elements reference',
          items: [
            { label: 'All elements', link: '/elements/' },
            {
              label: 'Content',
              items: [
                { label: 'div and block elements', link: '/elements/div/' },
                { label: 'span and inline elements', link: '/elements/span/' },
                { label: 'ul, ol and li', link: '/elements/lists/' },
                { label: 'br and wbr', link: '/elements/br/' },
                { label: 'ruby and rt', link: '/elements/ruby/' },
                { label: 'table', link: '/elements/table/' },
                { label: 'img', link: '/elements/img/' },
              ],
            },
            {
              label: 'Forms',
              items: [
                { label: 'button', link: '/elements/button/' },
                { label: 'input', link: '/elements/input/' },
                { label: 'textarea', link: '/elements/textarea/' },
                { label: 'select and option', link: '/elements/select/' },
                { label: 'meter and progress', link: '/elements/meter-progress/' },
                { label: 'details and summary', link: '/elements/details/' },
                { label: 'fieldset and legend', link: '/elements/fieldset/' },
                { label: 'form', link: '/elements/form/' },
              ],
            },
            { label: 'Media placeholders', link: '/elements/embedded/' },
          ],
        },
        {
          label: 'Styling reference',
          items: [
            { label: 'Properties', link: '/reference/properties/' },
            { label: 'Values, units and colors', link: '/reference/values/' },
            { label: 'Selectors and states', link: '/reference/selectors/' },
          ],
        },
        {
          label: 'App services',
          items: [
            { label: 'Windows', link: '/docs/services/windows/' },
            { label: 'Menus', link: '/docs/services/menus/' },
            { label: 'File dialogs', link: '/docs/services/dialogs/' },
            { label: 'Clipboard', link: '/docs/services/clipboard/' },
            { label: 'HTTP', link: '/docs/services/http/' },
            { label: 'Async tasks', link: '/docs/services/async/' },
            { label: 'Testing', link: '/docs/services/testing/' },
            { label: 'The CLI', link: '/docs/services/cli/' },
          ],
        },
        {
          label: 'Component library',
          items: [{ label: 'Components (in progress)', link: '/components/' }],
        },
        {
          label: 'Project',
          items: [
            { label: 'Rendering', link: '/docs/rendering/' },
            { label: 'Roadmap', link: '/docs/roadmap/' },
            { label: 'Blog', link: '/blog/' },
            { label: 'About', link: '/docs/about/' },
          ],
        },
      ],
    }),
  ],
});
