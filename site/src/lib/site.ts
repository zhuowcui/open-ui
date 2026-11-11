// Links shared by the landing page and the docs header.
export const base = import.meta.env.BASE_URL.replace(/\/?$/, '/');
export const repo = 'https://github.com/zhuowcui/open-ui';

export const nav = [
  { label: 'Docs', href: `${base}docs/` },
  { label: 'Components', href: `${base}components/` },
  { label: 'Blog', href: `${base}blog/` },
];

export const install =
  'cargo install --locked --git https://github.com/zhuowcui/open-ui openui-cli';
