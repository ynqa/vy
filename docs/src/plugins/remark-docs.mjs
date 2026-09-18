import path from 'node:path';
import { fileURLToPath } from 'node:url';

const contentDirectory = fileURLToPath(new URL('../content/docs/', import.meta.url));
const changelogFile = fileURLToPath(new URL('../../../CHANGELOG.md', import.meta.url));

// Keep GitHub-readable .md links in source, and emit base-aware site URLs at build time.
export function documentLink(url, sourceFile, base = '/') {
  if (/^(?:[a-z][\w+.-]*:|\/\/|#)/i.test(url)) return url;
  const match = url.match(/^([^?#]+\.mdx?)([?#].*)?$/);
  if (!match) return url;
  const target = path.resolve(path.dirname(sourceFile), match[1]);
  const relative = path.relative(contentDirectory, target);
  if (relative.startsWith('..') || path.isAbsolute(relative)) {
    throw new Error(`Document link is outside the content directory: ${url}`);
  }
  const route = relative.replaceAll(path.sep, '/').replace(/(?:^|\/)index\.mdx?$|\.mdx?$/g, '');
  return `${base.replace(/\/$/, '')}/${route}${route ? '/' : ''}${match[2] || ''}`;
}

export default function remarkDocs({ base = '/' } = {}) {
  return (tree, file) => {
    // Starlight supplies the page title when the root changelog is imported.
    if (file.path === changelogFile && tree.children[0]?.type === 'heading' && tree.children[0].depth === 1) {
      tree.children.shift();
    }
    function visit(node) {
      if (node.type === 'image' && node.url.startsWith('/demos/')) {
        node.url = `${base.replace(/\/$/, '')}${node.url}`;
      }
      if (node.type === 'link' || node.type === 'definition') {
        node.url = documentLink(node.url, file.path, base);
      }
      // Mermaid stays editable as a fenced Markdown block; render it only on pages with diagrams.
      if (node.type === 'code' && node.lang === 'mermaid') {
        const source = node.value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
        node.type = 'html';
        node.value = `<pre class="mermaid" tabindex="0" aria-label="Architecture diagram">${source}</pre>`;
      }
      node.children?.forEach(visit);
    }
    visit(tree);
  };
}
