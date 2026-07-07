/// Shared markdown rendering — used by DocumentsView, RoomsView, etc.
/// Renders markdown to sanitized HTML with GFM line breaks and
/// external links opening in the system browser.

import { marked } from 'marked';
import DOMPurify from 'dompurify';

// Custom renderer: external links get target=_blank for browser handling.
const renderer = new marked.Renderer();
renderer.link = ({ href, text }: { href: string; title?: string | null; text: string }) => {
  const url = href ?? '';
  if (url.startsWith('http://') || url.startsWith('https://')) {
    return `<a href="${url}" target="_blank" rel="noopener noreferrer">${text}</a>`;
  }
  return `<a href="${url}">${text}</a>`;
};

/** Render markdown to sanitized HTML.
 *  - GFM enabled (tables, strikethrough, task lists)
 *  - breaks: true (single \n → <br>)
 *  - External links get target=_blank
 */
export function renderMarkdown(md: string): string {
  try {
    const html = marked.parse(md, {
      renderer,
      gfm: true,
      breaks: true,
      async: false,
    }) as string;
    return DOMPurify.sanitize(html, {
      ADD_ATTR: ['target', 'rel'],
    });
  } catch {
    return DOMPurify.sanitize(md);
  }
}
