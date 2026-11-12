/**
 * Small, dependency-free Markdown renderer for Ollama output.
 *
 * Security: every character is HTML-escaped before any Markdown transform. We
 * only emit tags from a fixed allow-list, and links are restricted to http(s)
 * or mailto. Model output is treated as untrusted text, never raw HTML.
 */

export function escapeHtml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function inline(raw: string): string {
  let text = escapeHtml(raw);
  const code: string[] = [];
  text = text.replace(/`([^`]+)`/g, (_, value: string) => {
    const index = code.push(`<code>${value}</code>`) - 1;
    return `\u0000CODE${index}\u0000`;
  });
  text = text.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  text = text.replace(/__([^_]+)__/g, '<strong>$1</strong>');
  text = text.replace(/(^|[\s(])\*([^*\n]+)\*/g, '$1<em>$2</em>');
  text = text.replace(/(^|[\s(])_([^_\n]+)_/g, '$1<em>$2</em>');
  text = text.replace(/~~([^~]+)~~/g, '<del>$1</del>');
  text = text.replace(
    /\[([^\]]+)\]\((https?:\/\/[^\s)]+|mailto:[^\s)]+)\)/g,
    '<a href="$2" target="_blank" rel="noopener noreferrer">$1</a>'
  );
  text = text.replace(/\[(\d{1,3}:\d{2})\]/g, '<span class="timestamp">[$1]</span>');
  text = text.replace(/\u0000CODE(\d+)\u0000/g, (_, index: string) => code[Number(index)] ?? '');
  return text;
}

function isTableDivider(line: string): boolean {
  return /^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(line);
}

function renderTable(lines: string[]): string {
  if (lines.length < 2 || !isTableDivider(lines[1])) return '';
  const cells = (line: string) =>
    line
      .trim()
      .replace(/^\|/, '')
      .replace(/\|$/, '')
      .split('|')
      .map((cell) => cell.trim());
  const head = cells(lines[0]);
  const rows = lines.slice(2).map(cells);
  const thead = `<thead><tr>${head.map((cell) => `<th>${inline(cell)}</th>`).join('')}</tr></thead>`;
  const tbody = `<tbody>${rows
    .map((row) => `<tr>${row.map((cell) => `<td>${inline(cell)}</td>`).join('')}</tr>`)
    .join('')}</tbody>`;
  return `<div class="table-wrap"><table>${thead}${tbody}</table></div>`;
}

export function renderMarkdown(markdown: string): string {
  const source = (markdown || '').replace(/\r\n?/g, '\n');
  const lines = source.split('\n');
  const html: string[] = [];
  let index = 0;

  while (index < lines.length) {
    const line = lines[index];

    if (/^\s*```/.test(line)) {
      const language = line.replace(/^\s*```/, '').trim();
      const body: string[] = [];
      index++;
      while (index < lines.length && !/^\s*```/.test(lines[index])) {
        body.push(lines[index]);
        index++;
      }
      index++;
      html.push(
        `<pre class="code-block"${language ? ` data-lang="${escapeHtml(language)}"` : ''}><code>${escapeHtml(
          body.join('\n')
        )}</code></pre>`
      );
      continue;
    }

    if (!line.trim()) {
      index++;
      continue;
    }

    const heading = /^(#{1,4})\s+(.*)$/.exec(line);
    if (heading) {
      const level = Math.min(4, heading[1].length);
      html.push(`<h${level}>${inline(heading[2])}</h${level}>`);
      index++;
      continue;
    }

    if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) {
      html.push('<hr />');
      index++;
      continue;
    }

    if (/^\s*>/.test(line)) {
      const quote: string[] = [];
      while (index < lines.length && /^\s*>/.test(lines[index])) {
        quote.push(lines[index].replace(/^\s*>\s?/, ''));
        index++;
      }
      html.push(`<blockquote>${renderMarkdown(quote.join('\n'))}</blockquote>`);
      continue;
    }

    if (lines[index].includes('|') && index + 1 < lines.length && isTableDivider(lines[index + 1])) {
      const table: string[] = [lines[index], lines[index + 1]];
      index += 2;
      while (index < lines.length && lines[index].includes('|') && lines[index].trim()) {
        table.push(lines[index]);
        index++;
      }
      html.push(renderTable(table));
      continue;
    }

    if (/^\s*[-*+]\s+/.test(line) || /^\s*\d+\.\s+/.test(line)) {
      const ordered = /^\s*\d+\.\s+/.test(line);
      const items: string[] = [];
      const pattern = ordered ? /^\s*\d+\.\s+(.*)$/ : /^\s*[-*+]\s+(.*)$/;
      while (index < lines.length && pattern.test(lines[index])) {
        const match = pattern.exec(lines[index]);
        items.push(match ? match[1] : '');
        index++;
      }
      const list = items
        .map((item) => {
          const task = /^\[( |x|X)\]\s+(.*)$/.exec(item);
          if (task) {
            return `<li class="task"><input type="checkbox" disabled ${
              task[1].toLowerCase() === 'x' ? 'checked' : ''
            } /> ${inline(task[2])}</li>`;
          }
          return `<li>${inline(item)}</li>`;
        })
        .join('');
      html.push(`<${ordered ? 'ol' : 'ul'}>${list}</${ordered ? 'ol' : 'ul'}>`);
      continue;
    }

    const paragraph: string[] = [line];
    index++;
    while (
      index < lines.length &&
      lines[index].trim() &&
      !/^(#{1,4})\s+/.test(lines[index]) &&
      !/^\s*[-*+]\s+/.test(lines[index]) &&
      !/^\s*\d+\.\s+/.test(lines[index]) &&
      !/^\s*```/.test(lines[index]) &&
      !/^\s*>/.test(lines[index])
    ) {
      paragraph.push(lines[index]);
      index++;
    }
    html.push(`<p>${inline(paragraph.join(' '))}</p>`);
  }
  return html.join('\n');
}

export function titleFromMarkdown(markdown: string, fallback = 'Untitled note'): string {
  const match = /^#\s+(.+)$/m.exec(markdown || '');
  return (match?.[1] || fallback).replace(/[*_`]/g, '').trim().slice(0, 140);
}
