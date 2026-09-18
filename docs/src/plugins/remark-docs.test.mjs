import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import remarkDocs, { documentLink } from './remark-docs.mjs';

const source = fileURLToPath(new URL('../content/docs/reference/commands.md', import.meta.url));

test('resolves relative Markdown links, anchors, root pages, and deployment base', () => {
  assert.equal(documentLink('../explore/commands/jaq.md#edit-an-expression', source), '/explore/commands/jaq/#edit-an-expression');
  assert.equal(documentLink('settings/display.md', source, '/vy/'), '/vy/reference/settings/display/');
  assert.equal(documentLink('../index.md', source, '/vy/'), '/vy/');
  assert.equal(documentLink('../index.md', source), '/');
  assert.equal(documentLink('../changelog.mdx#unreleased', source, '/vy/'), '/vy/changelog/#unreleased');
});

test('omits the imported changelog title while preserving release headings and other documents', () => {
  const changelog = fileURLToPath(new URL('../../../CHANGELOG.md', import.meta.url));
  for (const file of [changelog, source]) {
    const tree = { type: 'root', children: [
      { type: 'heading', depth: 1, children: [] },
      { type: 'heading', depth: 2, children: [] },
    ] };
    remarkDocs()(tree, { path: file });
    assert.deepEqual(tree.children.map(({ depth }) => depth), file === changelog ? [2] : [1, 2]);
  }
});

test('preserves external URLs, fragments, and non-Markdown assets', () => {
  for (const url of ['https://example.com/README.md', '//example.com/a.md', '#here', '../diagram.svg']) {
    assert.equal(documentLink(url, source), url);
  }
  assert.throws(() => documentLink('../../../../README.md', source), /outside/);
});

test('prefixes demo images with the deployment base while preserving other images', () => {
  for (const base of ['/', '/vy/']) {
    const tree = { type: 'root', children: [
      { type: 'image', url: '/demos/explore/commands/jaq-run.gif' },
      { type: 'image', url: 'https://example.com/image.gif' },
      { type: 'image', url: './diagram.svg' },
    ] };
    remarkDocs({ base })(tree, { path: source });
    assert.equal(tree.children[0].url, `${base}demos/explore/commands/jaq-run.gif`);
    assert.equal(tree.children[1].url, 'https://example.com/image.gif');
    assert.equal(tree.children[2].url, './diagram.svg');
  }
});

test('transforms reference links and escapes Mermaid source without altering ordinary code', () => {
  const tree = { type: 'root', children: [
    { type: 'definition', url: '../explore/commands/focus.md' },
    { type: 'code', lang: 'mermaid', value: 'A["<script>&"] --> B' },
    { type: 'code', lang: 'sh', value: 'echo hello' },
  ] };
  remarkDocs({ base: '/vy/' })(tree, { path: source });
  assert.equal(tree.children[0].url, '/vy/explore/commands/focus/');
  assert.equal(tree.children[1].type, 'html');
  assert.match(tree.children[1].value, /&lt;script&gt;&amp;/);
  assert.equal(tree.children[2].type, 'code');
});
