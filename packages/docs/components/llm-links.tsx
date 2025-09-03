'use client';

import { ChevronDown, ExternalLink, FileText, MessageCircle, Sparkles } from 'lucide-react';
import { usePathname } from 'next/navigation';
import { ChatGPTIcon, ClaudeIcon, GitHubIcon } from '@/components/icons';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

interface LLMLinksProps {
  title: string;
  content: string;
  className?: string;
}

export function LLMLinks({ title, content, className = '' }: LLMLinksProps) {
  const pathname = usePathname();

  // Convert pathname to full URL for the markdown file
  const fullMarkdownUrl = `https://curl-runner.com${pathname === '/docs' ? '/docs/index.md' : `${pathname}.md`}`;

  // Create the encoded query for AI services
  const aiQuery = encodeURIComponent(`Read ${fullMarkdownUrl}, I want to ask questions about it.`);

  // URLs for different services
  const claudeUrl = `https://claude.ai/new?q=${aiQuery}`;
  const chatGPTUrl = `https://chatgpt.com/?hints=search&q=${aiQuery}`;
  const t3ChatUrl = `https://t3.chat/new?q=${aiQuery}`;
  const githubUrl = `https://github.com/alexvcasillas/curl-runner/blob/main/packages/docs/public${pathname === '/docs' ? '/docs/index.md' : `${pathname}.md`}`;

  // Convert pathname to markdown URL for local access
  const markdownUrl = pathname === '/docs' ? '/docs/index.md' : `${pathname}.md`;

  const menuItems = [
    {
      label: 'Ask Claude',
      url: claudeUrl,
      icon: ClaudeIcon,
      tooltip: 'Open this page in Claude AI',
    },
    {
      label: 'Ask ChatGPT',
      url: chatGPTUrl,
      icon: ChatGPTIcon,
      tooltip: 'Open this page in ChatGPT',
    },
    {
      label: 'Ask T3 Chat',
      url: t3ChatUrl,
      icon: MessageCircle,
      tooltip: 'Open this page in T3 Chat',
    },
    {
      label: 'Open in GitHub',
      url: githubUrl,
      icon: GitHubIcon,
      tooltip: 'View source on GitHub',
    },
  ];

  return (
    <div className={`flex items-center space-x-2 ${className}`}>
      <TooltipProvider>
        {/* Copy Markdown Button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button variant="ghost" size="sm" asChild>
              <a
                href={markdownUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center space-x-1"
              >
                <FileText className="h-3 w-3" />
                <span className="text-xs">Copy Markdown</span>
              </a>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>View Markdown version</p>
          </TooltipContent>
        </Tooltip>

        {/* Dropdown Menu */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="sm" className="flex items-center space-x-1">
              <span className="text-xs">Ask AI</span>
              <ChevronDown className="h-3 w-3" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-48">
            {menuItems.map((item) => (
              <DropdownMenuItem key={item.label} asChild>
                <a
                  href={item.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center space-x-2 cursor-pointer"
                >
                  <item.icon className="h-4 w-4" />
                  <span>{item.label}</span>
                  <ExternalLink className="h-3 w-3 ml-auto" />
                </a>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </TooltipProvider>
    </div>
  );
}
