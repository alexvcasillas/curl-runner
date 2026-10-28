import type { Item, Node } from 'fumadocs-core/page-tree';
import { getMarkdownUrl, source } from '@/lib/source';

export const revalidate = false;

type Section = { heading: string; items: string[] };

function entry(node: Item): string | undefined {
  // Sidebar links to pages outside the MDX source (not yet migrated) have no page: skip them.
  const page = source.getNodePage(node);
  if (!page) {
    return undefined;
  }
  const { title, description } = page.data;
  return `- [${title}](${getMarkdownUrl(page)})${description ? `: ${description}` : ''}`;
}

/** Walks the page tree in sidebar order; separators and folders open a new section. */
function collect(nodes: Node[], sections: Section[]): void {
  for (const node of nodes) {
    if (node.type === 'separator') {
      sections.push({ heading: String(node.name ?? ''), items: [] });
    } else if (node.type === 'folder') {
      sections.push({ heading: String(node.name), items: [] });
      if (node.index) {
        collect([node.index], sections);
      }
      collect(node.children, sections);
    } else {
      const line = entry(node);
      if (line) {
        sections[sections.length - 1].items.push(line);
      }
    }
  }
}

export function GET(): Response {
  const sections: Section[] = [{ heading: 'Docs', items: [] }];
  collect(source.getPageTree().children, sections);

  const body = [
    '# curl-runner',
    '',
    '> A CLI for running HTTP requests defined in YAML files: sequential or parallel execution, response assertions, variables and CI-friendly output. A single native binary.',
    '',
    'Every docs page is available as Markdown by appending `.md` to its URL.',
    ...sections
      .filter((section) => section.items.length > 0)
      .flatMap((section) => ['', `## ${section.heading}`, '', ...section.items]),
  ];

  return new Response(`${body.join('\n')}\n`, {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
}
