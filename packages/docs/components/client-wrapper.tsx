"use client"

import { TooltipProvider } from "@/components/ui/tooltip"

interface ClientWrapperProps {
  children: React.ReactNode
}

export function ClientWrapper({ children }: ClientWrapperProps) {
  return (
    <TooltipProvider>
      {children}
    </TooltipProvider>
  )
}