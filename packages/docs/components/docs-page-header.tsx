import { LLMLinks } from '@/components/llm-links';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';

interface DocsPageHeaderProps {
  heading: string;
  text?: string;
  badge?: string;
  children?: React.ReactNode;
}

export function DocsPageHeader({ heading, text, badge, children }: DocsPageHeaderProps) {
  return (
    <div className="space-y-6 pb-6">
      <div className="flex flex-col-reverse space-y-reverse space-y-4 sm:flex-row sm:items-start sm:justify-between sm:space-y-0">
        <div className="space-y-3 flex-1 sm:mr-4">
          <div className="flex flex-col space-y-2 sm:flex-row sm:items-center sm:space-y-0 sm:space-x-3">
            <h1 className="text-4xl font-bold tracking-tight">{heading}</h1>
            {badge && (
              <Badge variant="secondary" className="text-xs w-fit">
                {badge}
              </Badge>
            )}
          </div>
          {text && (
            <p className="text-xl text-muted-foreground leading-relaxed max-w-3xl">{text}</p>
          )}
        </div>
        <div className="flex-shrink-0">
          <LLMLinks title={heading} content={text || heading} />
        </div>
      </div>
      {children}
      <Separator className="my-8" />
    </div>
  );
}
