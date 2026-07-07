import { describe, it, expect } from 'vitest';
import { renderMarkdown } from './markdown';

describe('renderMarkdown', () => {
  it('renders basic markdown', () => {
    const html = renderMarkdown('# Hello');
    expect(html).toContain('<h1>');
    expect(html).toContain('Hello');
  });

  it('renders bold and italic', () => {
    const html = renderMarkdown('**bold** and *italic*');
    expect(html).toContain('<strong>bold</strong>');
    expect(html).toContain('<em>italic</em>');
  });

  it('converts single newline to <br> (GFM breaks)', () => {
    const html = renderMarkdown('line one\nline two');
    expect(html).toContain('<br>');
    expect(html).toContain('line one');
    expect(html).toContain('line two');
  });

  it('renders paragraphs for double newline', () => {
    const html = renderMarkdown('para one\n\npara two');
    expect(html).toContain('<p>');
  });

  it('adds target=_blank to external links', () => {
    const html = renderMarkdown('[Google](https://google.com)');
    expect(html).toContain('target="_blank"');
    expect(html).toContain('rel="noopener noreferrer"');
    expect(html).toContain('href="https://google.com"');
  });

  it('does not add target=_blank to internal links', () => {
    const html = renderMarkdown('[Child](./child-doc)');
    expect(html).toContain('href="./child-doc"');
    expect(html).not.toContain('target="_blank"');
  });

  it('renders code blocks', () => {
    const html = renderMarkdown('```\ncode here\n```');
    expect(html).toContain('<pre>');
    expect(html).toContain('<code>');
  });

  it('renders inline code', () => {
    const html = renderMarkdown('use `npm install` to setup');
    expect(html).toContain('<code>npm install</code>');
  });

  it('renders unordered lists', () => {
    const html = renderMarkdown('- item one\n- item two');
    expect(html).toContain('<ul>');
    expect(html).toContain('<li>item one</li>');
  });

  it('renders task lists (GFM)', () => {
    const html = renderMarkdown('- [x] done\n- [ ] todo');
    expect(html).toContain('checkbox');
  });

  it('renders tables (GFM)', () => {
    const html = renderMarkdown('| A | B |\n|---|---|\n| 1 | 2 |');
    expect(html).toContain('<table>');
  });

  it('handles empty input', () => {
    const html = renderMarkdown('');
    expect(html).toBe('');
  });

  it('sanitizes dangerous HTML', () => {
    const html = renderMarkdown('<script>alert("xss")</script>');
    expect(html).not.toContain('<script>');
  });
});
