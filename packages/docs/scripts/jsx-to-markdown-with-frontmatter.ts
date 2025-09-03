#!/usr/bin/env bun

import { mkdir } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';

interface CodeBlock {
  language: string;
  filename?: string;
  content: string;
}

interface TableData {
  headers: string[];
  rows: string[][];
}

interface PageMetadata {
  title: string;
  description: string;
  category: string;
  subcategory?: string;
  keywords: string[];
  slug: string;
  toc: boolean;
  related?: string[];
}

class EnhancedJSXToMarkdownConverter {
  private snippets: Map<string, string> = new Map();
  private codeExamples: Map<string, string> = new Map();
  private dataArrays: Map<string, any[]> = new Map();

  async convertFile(jsxPath: string, outputPath: string): Promise<void> {
    try {
      console.log(`Converting ${jsxPath}...`);
      const file = Bun.file(jsxPath);
      const content = await file.text();

      // Load snippets first
      await this.loadSnippets(jsxPath);

      // Extract code constants from the file
      this.extractCodeConstants(content);

      // Extract data arrays (like features)
      this.extractDataArrays(content);

      // Generate metadata
      const metadata = this.generateMetadata(jsxPath, content);

      // Convert to markdown with enhanced frontmatter
      const markdown = this.convertToMarkdown(content, metadata);

      // Ensure output directory exists
      const outputDir = dirname(outputPath);
      await mkdir(outputDir, { recursive: true });

      await Bun.write(outputPath, markdown);
      console.log(`✓ Converted to ${outputPath}`);
    } catch (error) {
      console.error(`✗ Error converting ${jsxPath}:`, error);
      throw error;
    }
  }

  private generateMetadata(filePath: string, content: string): PageMetadata {
    const pathParts = filePath.split('/');
    const fileName = pathParts[pathParts.length - 1].replace('.tsx', '');
    const parentDir = pathParts[pathParts.length - 2];

    // Extract basic info from content
    const heading = this.extractProp(content, 'DocsPageHeader', 'heading') || '';
    const description = this.extractProp(content, 'DocsPageHeader', 'text') || '';

    // Determine category and subcategory
    let category = 'Documentation';
    let subcategory: string | undefined;
    let slug = '/docs';

    if (parentDir === 'docs' && fileName === 'page') {
      category = 'Getting Started';
      slug = '/docs';
    } else if (parentDir === 'installation' || parentDir === 'quick-start') {
      category = 'Getting Started';
      slug = `/docs/${parentDir}`;
    } else if (parentDir === 'examples') {
      category = 'Examples';
      subcategory = fileName.charAt(0).toUpperCase() + fileName.slice(1);
      slug = `/docs/examples/${fileName}`;
    } else if (parentDir === 'features') {
      category = 'Features';
      subcategory = this.formatTitle(fileName);
      slug = `/docs/features/${fileName}`;
    } else if (parentDir === 'api-reference') {
      category = 'API Reference';
      subcategory = this.formatTitle(fileName);
      slug = `/docs/api-reference/${fileName}`;
    } else if (
      ['yaml-structure', 'variables', 'global-settings', 'cli-commands', 'cli-options'].includes(
        parentDir,
      )
    ) {
      category = 'Configuration';
      slug = `/docs/${parentDir}`;
    } else {
      slug = `/docs/${parentDir}`;
    }

    // Extract keywords from content
    const keywords = this.extractKeywords(heading, description, content);

    // Find related pages
    const related = this.findRelatedPages(fileName, category, content);

    return {
      title: heading,
      description,
      category,
      subcategory,
      keywords,
      slug,
      toc: true,
      related: related.length > 0 ? related : undefined,
    };
  }

  private extractKeywords(title: string, description: string, content: string): string[] {
    const keywords = new Set<string>();

    // Add base keywords
    keywords.add('curl-runner');
    keywords.add('http');
    keywords.add('api');
    keywords.add('testing');

    // Add title words (excluding common words)
    const titleWords = title
      .toLowerCase()
      .split(/\s+/)
      .filter((word) => word.length > 3 && !['with', 'from', 'the', 'and', 'for'].includes(word));
    titleWords.forEach((word) => keywords.add(word));

    // Add specific keywords based on content
    if (content.includes('YAML')) keywords.add('yaml');
    if (content.includes('variable')) keywords.add('variables');
    if (content.includes('template')) keywords.add('templating');
    if (content.includes('authentication')) keywords.add('authentication');
    if (content.includes('parallel')) keywords.add('parallel');
    if (content.includes('sequential')) keywords.add('sequential');
    if (content.includes('validation')) keywords.add('validation');
    if (content.includes('retry')) keywords.add('retry');
    if (content.includes('timeout')) keywords.add('timeout');
    if (content.includes('headers')) keywords.add('headers');
    if (content.includes('response')) keywords.add('response');
    if (content.includes('request')) keywords.add('request');
    if (content.includes('collection')) keywords.add('collection');
    if (content.includes('CLI') || content.includes('command')) keywords.add('cli');
    if (content.includes('install')) keywords.add('installation');
    if (content.includes('Docker')) keywords.add('docker');
    if (content.includes('Bun')) keywords.add('bun');
    if (content.includes('npm')) keywords.add('npm');
    if (content.includes('environment')) keywords.add('environment');

    return Array.from(keywords);
  }

  private findRelatedPages(currentPage: string, category: string, content: string): string[] {
    const related: string[] = [];

    // Define relationships
    const relationships: Record<string, string[]> = {
      installation: ['/docs/quick-start', '/docs/cli-commands'],
      'quick-start': ['/docs/installation', '/docs/yaml-structure', '/docs/examples/basic'],
      'yaml-structure': ['/docs/variables', '/docs/global-settings', '/docs/examples/basic'],
      variables: ['/docs/yaml-structure', '/docs/global-settings', '/docs/examples/advanced'],
      'global-settings': ['/docs/variables', '/docs/yaml-structure'],
      'cli-commands': ['/docs/cli-options', '/docs/quick-start'],
      'cli-options': ['/docs/cli-commands', '/docs/quick-start'],
      basic: ['/docs/examples/advanced', '/docs/examples/collection', '/docs/yaml-structure'],
      advanced: ['/docs/examples/basic', '/docs/variables', '/docs/features/parallel-execution'],
      collection: ['/docs/examples/advanced', '/docs/global-settings'],
      'request-object': [
        '/docs/api-reference/response-object',
        '/docs/api-reference/validation-rules',
      ],
      'response-object': [
        '/docs/api-reference/request-object',
        '/docs/api-reference/validation-rules',
      ],
      'validation-rules': [
        '/docs/api-reference/response-object',
        '/docs/features/response-validation',
      ],
      'parallel-execution': ['/docs/features/retry-mechanism', '/docs/global-settings'],
      'retry-mechanism': [
        '/docs/features/response-validation',
        '/docs/api-reference/request-object',
      ],
      'response-validation': [
        '/docs/api-reference/validation-rules',
        '/docs/features/retry-mechanism',
      ],
      'output-formats': ['/docs/cli-options', '/docs/global-settings'],
    };

    const pageKey = currentPage === 'page' ? 'index' : currentPage;
    return relationships[pageKey] || [];
  }

  private formatTitle(slug: string): string {
    return slug
      .split('-')
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  private convertToMarkdown(content: string, metadata: PageMetadata): string {
    const markdown: string[] = [];

    // Add comprehensive frontmatter
    markdown.push('---');
    markdown.push(`title: "${metadata.title}"`);
    markdown.push(`description: "${metadata.description.replace(/"/g, '\\"')}"`);
    markdown.push(`category: "${metadata.category}"`);
    if (metadata.subcategory) {
      markdown.push(`subcategory: "${metadata.subcategory}"`);
    }
    markdown.push(`keywords:`);
    metadata.keywords.forEach((keyword) => {
      markdown.push(`  - ${keyword}`);
    });
    markdown.push(`slug: "${metadata.slug}"`);
    markdown.push(`toc: ${metadata.toc}`);

    // Add metadata
    const now = new Date();
    markdown.push(`date: "${now.toISOString()}"`);
    markdown.push(`lastModified: "${now.toISOString()}"`);
    markdown.push(`author: "alexvcasillas"`);
    markdown.push(`authorUrl: "https://github.com/alexvcasillas/curl-runner"`);
    markdown.push(`license: "MIT"`);

    // Add navigation
    markdown.push(`nav:`);
    markdown.push(`  label: "${metadata.title}"`);
    if (metadata.category) {
      markdown.push(`  category: "${metadata.category}"`);
    }

    // Add related pages
    if (metadata.related && metadata.related.length > 0) {
      markdown.push(`related:`);
      metadata.related.forEach((page) => {
        markdown.push(`  - ${page}`);
      });
    }

    // Add tags
    markdown.push(`tags:`);
    markdown.push(`  - documentation`);
    markdown.push(`  - ${metadata.category.toLowerCase().replace(/\s+/g, '-')}`);
    if (metadata.subcategory) {
      markdown.push(`  - ${metadata.subcategory.toLowerCase().replace(/\s+/g, '-')}`);
    }

    // Add OpenGraph metadata
    markdown.push(`og:`);
    markdown.push(`  title: "${metadata.title} - curl-runner Documentation"`);
    markdown.push(`  description: "${metadata.description.replace(/"/g, '\\"')}"`);
    markdown.push(`  type: "article"`);
    markdown.push(`  image: "/og-image.png"`);

    // Add schema.org metadata
    markdown.push(`schema:`);
    markdown.push(`  "@context": "https://schema.org"`);
    markdown.push(`  "@type": "TechArticle"`);
    markdown.push(`  headline: "${metadata.title}"`);
    markdown.push(`  description: "${metadata.description.replace(/"/g, '\\"')}"`);
    markdown.push(`  datePublished: "${now.toISOString()}"`);
    markdown.push(`  dateModified: "${now.toISOString()}"`);

    markdown.push('---');
    markdown.push('');

    // Add main heading
    if (metadata.title) {
      markdown.push(`# ${metadata.title}`);
      markdown.push('');
    }

    // Add description if present
    if (metadata.description) {
      markdown.push(metadata.description);
      markdown.push('');
    }

    // Process all sections
    const sections = this.extractSections(content);
    for (const section of sections) {
      markdown.push(section);
      markdown.push('');
    }

    return markdown.join('\n').trim() + '\n';
  }

  // ... (rest of the methods remain the same as in jsx-to-markdown-complete.ts)

  private extractSections(content: string): string[] {
    const sections: string[] = [];

    // Match all <section> tags with their content
    const sectionRegex = /<section[^>]*>([\s\S]*?)<\/section>/g;
    let match;

    while ((match = sectionRegex.exec(content)) !== null) {
      const sectionContent = match[1];
      const sectionMarkdown = this.processSection(sectionContent);
      if (sectionMarkdown) {
        sections.push(sectionMarkdown);
      }
    }

    return sections;
  }

  private processSection(content: string): string {
    const result: string[] = [];

    // Extract main heading (H2)
    const h2Match = content.match(/<(?:H2|h2)(?:\s+[^>]*)?>([^<]+)<\/(?:H2|h2)>/);
    const sectionTitle = h2Match ? this.cleanText(h2Match[1]) : '';

    if (h2Match) {
      result.push(`## ${sectionTitle}`);
      result.push('');
    }

    // Extract paragraphs that are direct children of section
    const topLevelParagraphs = this.extractTopLevelParagraphs(content);
    for (const para of topLevelParagraphs) {
      result.push(this.convertInlineElements(para));
      result.push('');
    }

    // Handle section-specific content
    if (sectionTitle === 'Features') {
      // For Features section, extract features from data arrays
      const features = this.dataArrays.get('features');
      if (features && features.length > 0) {
        for (const feature of features) {
          result.push(`### ${feature.title}`);
          result.push('');
          result.push(feature.description);
          result.push('');
        }
      }
    } else if (sectionTitle === 'Next Steps') {
      // For Next Steps section, use hardcoded content that matches the JSX
      result.push('### Installation Guide');
      result.push('');
      result.push('Detailed installation instructions for all platforms');
      result.push('');
      result.push('[Get Started](/docs/installation)');
      result.push('');

      result.push('### YAML Structure');
      result.push('');
      result.push('Learn the YAML configuration format and options');
      result.push('');
      result.push('[Learn More](/docs/yaml-structure)');
      result.push('');

      result.push('### Examples');
      result.push('');
      result.push('Browse real-world examples and use cases');
      result.push('');
      result.push('[Explore](/docs/examples/basic)');
      result.push('');
    } else if (sectionTitle === 'Best Practices') {
      // For Best Practices section in Variables page, add the card content
      result.push('### Best Practices');
      result.push('');
      result.push('• Use descriptive variable names');
      result.push('• Define common values as variables');
      result.push('• Use environment variables for secrets');
      result.push('• Group related variables logically');
      result.push('• Document complex expressions');
      result.push('');
    }

    // Process subsections (divs with content)
    const subsections = this.extractSubsections(content);
    for (const subsection of subsections) {
      result.push(subsection);
      result.push('');
    }

    // Tables are now extracted at the subsection level to prevent duplicates

    // Extract lists that are direct children
    const lists = this.extractLists(content);
    for (const list of lists) {
      result.push(list);
      result.push('');
    }

    // Extract standalone code blocks
    const codeBlocks = this.extractCodeBlocks(content);
    for (const block of codeBlocks) {
      result.push(this.formatCodeBlock(block));
      result.push('');
    }

    // Clean up any leftover JSX template variables
    let finalResult = result.join('\n').trim();
    finalResult = finalResult.replace(/\{feature\.title\}/g, '');
    finalResult = finalResult.replace(/\{feature\.description\}/g, '');
    finalResult = finalResult.replace(/### \n\n/g, ''); // Remove empty headings

    return finalResult;
  }

  private extractCardGrids(content: string): string[] {
    const results: string[] = [];

    // Match grid containers that contain cards (various patterns)
    const gridPatterns = [
      /<div className="grid gap-4[^"]*"[^>]*>([\s\S]*?)<\/div>/g,
      /<div className="grid gap-6[^"]*"[^>]*>([\s\S]*?)<\/div>/g,
    ];

    for (const pattern of gridPatterns) {
      let match;
      while ((match = pattern.exec(content)) !== null) {
        const gridContent = match[1];
        const cards = this.extractCards(gridContent);

        if (cards.length > 0) {
          results.push(...cards);
        }
      }
    }

    // Feature cards are handled separately in processSection for Features section

    return results;
  }

  private extractCards(content: string): string[] {
    const cards: string[] = [];

    // Match individual card divs - simpler pattern
    const cardRegex =
      /<div className="rounded-lg border bg-card[^"]*"[^>]*>([\s\S]+?)<\/div>\s*<\/div>/g;
    let match;

    while ((match = cardRegex.exec(content)) !== null) {
      const cardContent = match[1];
      const cardMarkdown = this.processCard(cardContent);
      if (cardMarkdown) {
        cards.push(cardMarkdown);
      }
    }

    return cards;
  }

  private processCard(content: string): string {
    const result: string[] = [];

    // Extract card title (h4) - handle both direct h4 and nested h4
    const titleMatch = content.match(/<h4[^>]*>([\s\S]*?)<\/h4>/);
    if (titleMatch) {
      const title = this.cleanText(titleMatch[1]);
      result.push(`### ${title}`);
      result.push('');
    }

    // Extract card description (p tags) - handle various class patterns
    const descPatterns = [
      /<p[^>]*class="[^"]*text-sm text-muted-foreground[^"]*"[^>]*>([\s\S]*?)<\/p>/,
      /<p[^>]*class="[^"]*text-muted-foreground[^"]*"[^>]*>([\s\S]*?)<\/p>/,
    ];

    for (const pattern of descPatterns) {
      const descMatch = content.match(pattern);
      if (descMatch) {
        const description = this.cleanText(descMatch[1]);
        result.push(description);
        result.push('');
        break;
      }
    }

    // Extract link from Button > Link component
    const linkMatch = content.match(/<Link href="([^"]*)"[^>]*>([\s\S]*?)<\/Link>/);
    if (linkMatch) {
      const href = linkMatch[1];
      const linkTextMatch = linkMatch[2].match(/([^<]*?)(?:\s*<|$)/);
      const linkText = linkTextMatch ? this.cleanText(linkTextMatch[1]).trim() : 'Learn more';
      result.push(`[${linkText}](${href})`);
      result.push('');
    }

    return result.join('\n').trim();
  }

  private extractFeatureCards(content: string): string[] {
    const cards: string[] = [];

    // First check if we have extracted features data
    const features = this.dataArrays.get('features');
    if (features && features.length > 0) {
      for (const feature of features) {
        const result: string[] = [];
        result.push(`### ${feature.title}`);
        result.push('');
        result.push(feature.description);
        result.push('');
        cards.push(result.join('\n').trim());
      }
      return cards;
    }

    // Fallback: Look for feature cards with different structure (rounded-lg border bg-card p-6)
    const featureRegex =
      /<div[^>]*className="[^"]*rounded-lg border bg-card[^"]*"[^>]*>([\s\S]*?)<\/div>/g;
    let match;

    while ((match = featureRegex.exec(content)) !== null) {
      const cardContent = match[1];

      // Look for h3 title within feature cards
      const titleMatch = cardContent.match(/<h3[^>]*>([\s\S]*?)<\/h3>/);
      // Look for description paragraph
      const descMatch = cardContent.match(
        /<p[^>]*class="[^"]*text-muted-foreground[^"]*"[^>]*>([\s\S]*?)<\/p>/,
      );

      if (titleMatch || descMatch) {
        const result: string[] = [];

        if (titleMatch) {
          const title = this.cleanText(titleMatch[1]);
          result.push(`### ${title}`);
          result.push('');
        }

        if (descMatch) {
          const description = this.cleanText(descMatch[1]);
          result.push(description);
          result.push('');
        }

        cards.push(result.join('\n').trim());
      }
    }

    return cards;
  }

  private extractSubsections(content: string): string[] {
    const subsections: string[] = [];

    // Match divs that contain H3 headings (subsections)
    const divRegex = /<div(?:\s+className="[^"]*")?>([\s\S]*?)<\/div>/g;
    let match;

    while ((match = divRegex.exec(content)) !== null) {
      const divContent = match[1];

      // Skip if this is already processed as part of a card grid
      if (
        divContent.includes('CardContent') ||
        divContent.includes('CardTitle') ||
        divContent.includes('grid gap-4') ||
        divContent.includes('grid-cols')
      ) {
        continue;
      }

      // Check if this div has an H3
      const h3Match = divContent.match(
        /<(?:H3|h3)(?:\s+[^>]*)?>([^<]*(?:<Badge[^>]*>[^<]*<\/Badge>)?[^<]*)<\/(?:H3|h3)>/,
      );

      if (h3Match) {
        const subsectionResult: string[] = [];

        // Extract H3 title with any badges
        let h3Title = h3Match[1];
        h3Title = h3Title.replace(/<Badge[^>]*>(.*?)<\/Badge>/g, ' _$1_');
        h3Title = this.cleanText(h3Title);
        subsectionResult.push(`### ${h3Title}`);
        subsectionResult.push('');

        // Extract paragraph after H3
        const paraMatch = divContent.match(/<p[^>]*>([\s\S]*?)<\/p>/);
        if (paraMatch) {
          subsectionResult.push(this.convertInlineElements(paraMatch[1]));
          subsectionResult.push('');
        }

        // Extract code block in this subsection
        const codeBlockMatch = divContent.match(
          /<CodeBlockServer(?:\s+[^>]+)?>([\s\S]*?)<\/CodeBlockServer>/,
        );
        if (codeBlockMatch) {
          const codeBlock = this.parseCodeBlock(codeBlockMatch[0]);
          if (codeBlock) {
            subsectionResult.push(this.formatCodeBlock(codeBlock));
            subsectionResult.push('');
          }
        }

        // Extract tables in this subsection
        const tables = this.extractTables(divContent);
        for (const table of tables) {
          subsectionResult.push(this.formatTable(table));
          subsectionResult.push('');
        }

        subsections.push(subsectionResult.join('\n').trim());
      } else if (divContent.includes('rounded-lg border')) {
        // This is a special info/alert box
        const boxContent = this.processInfoBox(divContent);
        if (boxContent) {
          subsections.push(boxContent);
        }
      }
    }

    return subsections;
  }

  private processInfoBox(content: string): string {
    const result: string[] = [];

    // Extract heading (h4 or similar)
    const headingMatch = content.match(/<h4[^>]*>(.*?)<\/h4>/);
    if (headingMatch) {
      result.push(`> **${this.convertInlineElements(headingMatch[1])}**`);
    }

    // Extract paragraphs
    const paraRegex = /<p[^>]*>([\s\S]*?)<\/p>/g;
    let match;
    while ((match = paraRegex.exec(content)) !== null) {
      const text = this.convertInlineElements(match[1]);
      if (text) {
        result.push(`> ${text}`);
      }
    }

    // Special handling for precedence order or similar structures
    if (content.includes('Precedence Order')) {
      const items = content.matchAll(
        /<Badge[^>]*>(\d+)<\/Badge>[\s\S]*?<span[^>]*>(.*?)<\/span>[\s\S]*?<code[^>]*>(.*?)<\/code>/g,
      );
      result.push('>');
      for (const item of items) {
        result.push(
          `> ${item[1]}. **${this.cleanText(item[2])}** - \`${this.convertInlineElements(item[3])}\``,
        );
      }
    }

    // Extract lists of code items
    const codeItems = content.matchAll(
      /<code[^>]*>(.*?)<\/code>.*?[-–]\s*(.*?)(?=<div|<\/div|$)/gs,
    );
    for (const item of codeItems) {
      if (!result.some((r) => r.includes(item[1]))) {
        result.push(`> - \`${this.convertInlineElements(item[1])}\` - ${this.cleanText(item[2])}`);
      }
    }

    return result.join('\n');
  }

  private extractTopLevelParagraphs(content: string): string[] {
    const paragraphs: string[] = [];

    // Remove nested divs first to avoid extracting their paragraphs
    const contentWithoutDivs = content.replace(/<div[^>]*>[\s\S]*?<\/div>/g, '');

    // Now extract paragraphs
    const paraRegex = /<p(?:\s+className="[^"]*")?>([\s\S]*?)<\/p>/g;
    let match;

    while ((match = paraRegex.exec(contentWithoutDivs)) !== null) {
      paragraphs.push(match[1]);
    }

    return paragraphs;
  }

  private extractTables(content: string): TableData[] {
    const tables: TableData[] = [];
    const tableRegex = /<table[^>]*>([\s\S]*?)<\/table>/g;
    let match;

    while ((match = tableRegex.exec(content)) !== null) {
      const tableContent = match[1];
      const table: TableData = {
        headers: [],
        rows: [],
      };

      // Extract headers
      const theadMatch = tableContent.match(/<thead[^>]*>([\s\S]*?)<\/thead>/);
      if (theadMatch) {
        const thRegex = /<th[^>]*>([\s\S]*?)<\/th>/g;
        let thMatch;
        while ((thMatch = thRegex.exec(theadMatch[1])) !== null) {
          table.headers.push(this.convertInlineElements(thMatch[1]));
        }
      }

      // Extract rows
      const tbodyMatch = tableContent.match(/<tbody[^>]*>([\s\S]*?)<\/tbody>/);
      if (tbodyMatch) {
        const trRegex = /<tr[^>]*>([\s\S]*?)<\/tr>/g;
        let trMatch;

        while ((trMatch = trRegex.exec(tbodyMatch[1])) !== null) {
          const row: string[] = [];
          const tdRegex = /<td[^>]*>([\s\S]*?)<\/td>/g;
          let tdMatch;

          while ((tdMatch = tdRegex.exec(trMatch[1])) !== null) {
            row.push(this.convertInlineElements(tdMatch[1]));
          }

          if (row.length > 0) {
            table.rows.push(row);
          }
        }
      }

      if (table.headers.length > 0) {
        tables.push(table);
      }
    }

    return tables;
  }

  private formatTable(table: TableData): string {
    if (table.headers.length === 0) return '';

    const lines: string[] = [];

    // Add headers
    lines.push('| ' + table.headers.join(' | ') + ' |');
    lines.push('|' + table.headers.map(() => ' --- ').join('|') + '|');

    // Add rows
    for (const row of table.rows) {
      lines.push('| ' + row.join(' | ') + ' |');
    }

    return lines.join('\n');
  }

  private extractLists(content: string): string[] {
    const lists: string[] = [];

    // Remove nested divs to avoid extracting their lists
    const contentWithoutDivs = content.replace(/<div[^>]*>[\s\S]*?<\/div>/g, '');

    const listRegex = /<(ul|ol)[^>]*>([\s\S]*?)<\/\1>/g;
    let match;

    while ((match = listRegex.exec(contentWithoutDivs)) !== null) {
      const isOrdered = match[1] === 'ol';
      const listContent = match[2];
      const items: string[] = [];

      const itemRegex = /<li[^>]*>([\s\S]*?)<\/li>/g;
      let itemMatch;

      while ((itemMatch = itemRegex.exec(listContent)) !== null) {
        items.push(this.convertInlineElements(itemMatch[1]));
      }

      if (items.length > 0) {
        const formattedList = items
          .map((item, index) => (isOrdered ? `${index + 1}. ${item}` : `- ${item}`))
          .join('\n');
        lists.push(formattedList);
      }
    }

    return lists;
  }

  private extractCodeBlocks(content: string): CodeBlock[] {
    const blocks: CodeBlock[] = [];

    // Match CodeBlockServer components with all their content
    const codeBlockRegex = /<CodeBlockServer(?:\s+[^>]+)?>([\s\S]*?)<\/CodeBlockServer>/g;
    let match;

    while ((match = codeBlockRegex.exec(content)) !== null) {
      const block = this.parseCodeBlock(match[0]);
      if (block && !this.isDuplicateCodeBlock(blocks, block)) {
        blocks.push(block);
      }
    }

    return blocks;
  }

  private parseCodeBlock(fullMatch: string): CodeBlock | null {
    // Extract props
    const langMatch = fullMatch.match(/language="([^"]+)"/);
    const fileMatch = fullMatch.match(/filename="([^"]+)"/);

    const language = langMatch ? langMatch[1] : 'text';
    const filename = fileMatch ? fileMatch[1] : undefined;

    // Extract content between tags
    const contentMatch = fullMatch.match(/<CodeBlockServer[^>]*>([\s\S]*?)<\/CodeBlockServer>/);
    if (!contentMatch) return null;

    const innerContent = contentMatch[1];
    let codeContent = '';

    // Check for variable reference {variableName}
    const varMatch = innerContent.match(/\{(\w+)\}/);
    if (varMatch) {
      const varName = varMatch[1];
      // First check code examples, then snippets
      codeContent = this.codeExamples.get(varName) || this.snippets.get(varName) || '';
    } else {
      // Check for template literal {`...`}
      const templateMatch = innerContent.match(/\{`([^`]*)`\}/s);
      if (templateMatch) {
        codeContent = templateMatch[1];
      } else {
        // Direct text content (remove any JSX tags)
        codeContent = innerContent.replace(/<[^>]+>/g, '').trim();
      }
    }

    if (!codeContent) return null;

    return {
      language,
      filename,
      content: codeContent,
    };
  }

  private isDuplicateCodeBlock(blocks: CodeBlock[], newBlock: CodeBlock): boolean {
    return blocks.some(
      (b) =>
        b.content === newBlock.content &&
        b.language === newBlock.language &&
        b.filename === newBlock.filename,
    );
  }

  private formatCodeBlock(block: CodeBlock): string {
    const lines: string[] = [];

    if (block.filename) {
      lines.push(`**${block.filename}**`);
      lines.push('');
    }

    lines.push(`\`\`\`${block.language}`);
    lines.push(block.content.trim());
    lines.push('```');

    return lines.join('\n');
  }

  private convertInlineElements(text: string): string {
    if (!text) return '';

    return (
      text
        // Handle template literal syntax
        .replace(/\$\{`\{([^}]+)\}`\}/g, '${$1}')
        .replace(/\{`([^`]+)`\}/g, '$1')
        .replace(/\{['"]([^'"]+)['"]\}/g, '$1')
        // Handle JSX components
        .replace(/<code(?:\s+[^>]*)?>([\s\S]*?)<\/code>/g, '`$1`')
        .replace(/<strong(?:\s+[^>]*)?>([\s\S]*?)<\/strong>/g, '**$1**')
        .replace(/<b(?:\s+[^>]*)?>([\s\S]*?)<\/b>/g, '**$1**')
        .replace(/<em(?:\s+[^>]*)?>([\s\S]*?)<\/em>/g, '*$1*')
        .replace(/<i(?:\s+[^>]*)?>([\s\S]*?)<\/i>/g, '*$1*')
        .replace(/<Badge(?:\s+[^>]*)?>([\s\S]*?)<\/Badge>/g, '_$1_')
        .replace(/<Link\s+href="([^"]+)"[^>]*>([\s\S]*?)<\/Link>/g, '[$2]($1)')
        .replace(/<a\s+href="([^"]+)"[^>]*>([\s\S]*?)<\/a>/g, '[$2]($1)')
        // Handle icons
        .replace(/<Info[^>]*\/>/g, 'ℹ️')
        .replace(/<CheckCircle[^>]*\/>/g, '✓')
        .replace(/<AlertTriangle[^>]*\/>/g, '⚠️')
        .replace(/<AlertCircle[^>]*\/>/g, '⚠️')
        // Remove attributes
        .replace(/\s*className="[^"]*"/g, '')
        .replace(/\s*variant="[^"]*"/g, '')
        .replace(/\s*id="[^"]*"/g, '')
        // Clean remaining tags
        .replace(/<[^>]+>/g, '')
        // Fix whitespace
        .replace(/\s+/g, ' ')
        .trim()
    );
  }

  private extractProp(content: string, component: string, prop: string): string {
    const regex = new RegExp(`<${component}[^>]*\\s${prop}="([^"]+)"`, 's');
    const match = content.match(regex);
    return match ? match[1] : '';
  }

  private cleanText(text: string): string {
    return text.replace(/\s+/g, ' ').trim();
  }

  private extractDataArrays(content: string): void {
    // Extract features array
    const featuresMatch = content.match(/const features = \[([\s\S]*?)\];/);
    if (featuresMatch) {
      try {
        // Parse the features array manually since it contains complex objects
        const featuresCode = featuresMatch[1];
        const features: any[] = [];

        // Extract individual feature objects
        const featureMatches = featuresCode.match(
          /\{[^}]*title:\s*['"](.*?)['"][^}]*description:\s*['"]([\s\S]*?)['"][^}]*\}/g,
        );
        if (featureMatches) {
          for (const featureMatch of featureMatches) {
            const titleMatch = featureMatch.match(/title:\s*['"](.*?)['"]/);
            const descMatch = featureMatch.match(/description:\s*['"]([\s\S]*?)['"]/);

            if (titleMatch && descMatch) {
              features.push({
                title: titleMatch[1],
                description: descMatch[1].replace(/\\n/g, ' ').trim(),
              });
            }
          }
        }

        this.dataArrays.set('features', features);
      } catch (error) {
        console.warn('Failed to parse features array:', error);
      }
    }
  }

  private extractCodeConstants(content: string) {
    // Extract const declarations with template literals
    const constRegex = /const\s+(\w+)\s*=\s*`([^`]*)`/gs;
    let match;

    while ((match = constRegex.exec(content)) !== null) {
      this.codeExamples.set(match[1], match[2]);
    }

    // Extract const declarations with regular strings
    const stringRegex = /const\s+(\w+)\s*=\s*["']([^"']+)["']/g;
    while ((match = stringRegex.exec(content)) !== null) {
      this.codeExamples.set(match[1], match[2]);
    }
  }

  private async loadSnippets(filePath: string) {
    const dir = dirname(filePath);
    const snippetsPath = join(dir, 'snippets.ts');

    try {
      const file = Bun.file(snippetsPath);
      if (await file.exists()) {
        const content = await file.text();

        // Parse export const declarations with template literals
        const templateRegex = /export\s+const\s+(\w+)\s*=\s*`([^`]*)`/gs;
        let match;
        while ((match = templateRegex.exec(content)) !== null) {
          this.snippets.set(match[1], match[2]);
        }

        // Parse export const declarations with regular strings
        const stringRegex = /export\s+const\s+(\w+)\s*=\s*["']([^"']+)["']/g;
        while ((match = stringRegex.exec(content)) !== null) {
          this.snippets.set(match[1], match[2]);
        }

        console.log(`  Loaded ${this.snippets.size} snippets from ${snippetsPath}`);
      }
    } catch (error) {
      console.warn(`  Could not load snippets from ${snippetsPath}`);
    }
  }
}

// Main conversion function
async function convertAllDocs() {
  const converter = new EnhancedJSXToMarkdownConverter();
  const docsDir = join(process.cwd(), 'app', '(docs)', 'docs');
  const outputDir = join(process.cwd(), 'public', 'docs');

  console.log('Starting JSX to Markdown conversion with enhanced frontmatter...\n');
  console.log('Scanning for all page.tsx files in:', docsDir);

  // Use Bun's glob to find all page.tsx files
  const glob = new Bun.Glob('**/page.tsx');
  const pageFiles: string[] = [];

  for await (const file of glob.scan({ cwd: docsDir, absolute: true })) {
    pageFiles.push(file);
  }

  console.log(`Found ${pageFiles.length} page(s) to convert\n`);

  let successCount = 0;
  let failureCount = 0;

  for (const pageFile of pageFiles) {
    // Calculate the relative path from docsDir
    const relativePath = relative(docsDir, pageFile);

    // Convert page.tsx to .md
    const outputFileName = relativePath.replace('/page.tsx', '.md').replace('page.tsx', 'index.md');

    // Build the full output path
    const outputPath = join(outputDir, outputFileName);

    try {
      await converter.convertFile(pageFile, outputPath);
      successCount++;
    } catch (error) {
      console.error(`Failed to convert ${pageFile}:`, error);
      failureCount++;
    }
  }

  console.log('\n' + '='.repeat(50));
  console.log(`✨ Conversion complete with enhanced frontmatter!`);
  console.log(`✓ Successfully converted: ${successCount} pages`);
  if (failureCount > 0) {
    console.log(`✗ Failed: ${failureCount} pages`);
  }
  console.log('='.repeat(50));
}

// Run the conversion
convertAllDocs().catch(console.error);
