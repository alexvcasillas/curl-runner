'use client';

import { Bot, MessageCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

interface LLMLinksProps {
  title: string;
  content: string;
  className?: string;
}

export function LLMLinks({ title, content, className = '' }: LLMLinksProps) {
  const encodedContent = encodeURIComponent(
    `Please help me understand this curl-runner documentation:\n\nTitle: ${title}\n\nContent:\n${content}`,
  );

  const claudeUrl = `https://claude.ai/chat?q=${encodedContent}`;
  const chatGPTUrl = `https://chat.openai.com/?q=${encodedContent}`;

  return (
    <div className={`flex items-center space-x-1 ${className}`}>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              asChild
              className="h-8 px-2 text-primary hover:text-primary"
            >
              <a
                href={claudeUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center space-x-1"
              >
                <Bot className="h-3 w-3" />
                <span className="text-xs">Claude</span>
              </a>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>Open this page in Claude</p>
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              asChild
              className="h-8 px-2 text-primary hover:text-primary"
            >
              <a
                href={chatGPTUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center space-x-1"
              >
                <MessageCircle className="h-3 w-3" />
                <span className="text-xs">ChatGPT</span>
              </a>
            </Button>
          </TooltipTrigger>
          <TooltipContent>
            <p>Open this page in ChatGPT</p>
          </TooltipContent>
        </Tooltip>
      </TooltipProvider>
    </div>
  );
}
