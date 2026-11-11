// Turn the `code` spans in a generated table cell into <code> elements.
// Everything else is escaped, so the cell can be set with set:html.
const escape = (text: string) =>
  text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

export function inline(text: string): string {
  return text
    .split('`')
    .map((part, i) => (i % 2 ? `<code>${escape(part)}</code>` : escape(part).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')))
    .join('');
}
