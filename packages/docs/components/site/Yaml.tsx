import { codeToHtml } from 'shiki';
import { YAML_THEME } from '@/lib/shiki-theme';

export { YAML_THEME };

export async function Yaml({ source }: { source: string }) {
  const html = await codeToHtml(source.trim(), { lang: 'yaml', theme: YAML_THEME });

  return (
    <div
      className="min-w-0 font-mono text-[13.5px] leading-[1.75] text-fg [&_code]:font-mono [&_pre]:m-0 [&_pre]:overflow-x-auto [&_pre]:px-[22px] [&_pre]:py-5 [&_pre]:focus-visible:outline-2 [&_pre]:focus-visible:-outline-offset-2 [&_pre]:focus-visible:outline-brand"
      // biome-ignore lint/security/noDangerouslySetInnerHtml: Shiki output from a trusted, build-time string
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
