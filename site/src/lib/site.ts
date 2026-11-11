// Links shared by the landing page and the docs header.
export { name, repo, install, siteUrl } from './project.mjs';

export const base = import.meta.env.BASE_URL.replace(/\/?$/, '/');

export const nav = [
  { label: 'Docs', href: `${base}docs/` },
  { label: 'Elements', href: `${base}elements/` },
  { label: 'Components', href: `${base}components/` },
  { label: 'Blog', href: `${base}blog/` },
];
