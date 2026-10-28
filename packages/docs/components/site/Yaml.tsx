import { codeToHtml, type ThemeRegistration } from 'shiki';

const YAML_THEME: ThemeRegistration = {
  name: 'curl-runner-monochrome',
  type: 'dark',
  fg: '#EDEDED',
  bg: 'transparent',
  settings: [
    { settings: { foreground: '#EDEDED', background: 'transparent' } },
    { scope: ['entity.name.tag'], settings: { foreground: '#22D3EE' } },
    { scope: ['string.quoted'], settings: { foreground: '#F5A97F' } },
    { scope: ['comment'], settings: { foreground: '#8A8A8A' } },
  ],
};

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
