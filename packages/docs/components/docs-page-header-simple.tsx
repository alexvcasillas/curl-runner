import { Badge } from "@/components/ui/badge"
import { Separator } from "@/components/ui/separator"

interface DocsPageHeaderSimpleProps {
  heading: string
  text?: string
  badge?: string
  children?: React.ReactNode
}

export function DocsPageHeaderSimple({
  heading,
  text,
  badge,
  children,
}: DocsPageHeaderSimpleProps) {
  return (
    <div className="space-y-6 pb-6">
      <div className="space-y-3">
        <div className="flex items-center space-x-3">
          <h1 className="text-4xl font-bold tracking-tight">{heading}</h1>
          {badge && <Badge variant="secondary" className="text-xs">{badge}</Badge>}
        </div>
        {text && <p className="text-xl text-muted-foreground leading-relaxed max-w-3xl">{text}</p>}
      </div>
      {children}
      <Separator className="my-8" />
    </div>
  )
}