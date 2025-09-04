'use client';

import { Bookmark, BookOpenIcon, ChevronRightIcon, Github, List } from 'lucide-react';
import { SiteFooter } from '@/components/site-footer';

const links = [
  {
    name: 'Documentation',
    href: '/docs',
    description: 'Learn how to integrate curl-runner with your app.',
    icon: BookOpenIcon,
  },
  {
    name: 'API Reference',
    href: '/docs/api-reference/request-object',
    description: 'A complete API reference for curl-runner.',
    icon: List,
  },
  {
    name: 'Guides',
    href: '/docs/quick-start',
    description: 'Installation guides that cover popular setups.',
    icon: Bookmark,
  },
  {
    name: 'Github',
    href: 'https://github.com/alexvcasillas/curl-runner',
    description: 'View the source code on GitHub.',
    icon: Github,
  },
];

export default function ErrorPage({ error, reset }: { error: Error; reset: () => void }) {
  return (
    <>
      <main className="mx-auto w-full max-w-7xl px-6 pt-10 pb-16 sm:pb-24 lg:px-8">
        <img
          alt="curl-runner"
          src="/icon-light-192.png"
          className="mx-auto h-10 w-auto sm:h-12 dark:block"
        />
        <img
          alt="curl-runner"
          src="/icon-dark-192.png"
          className="mx-auto h-10 w-auto hidden dark:hidden sm:h-12"
        />
        <div className="mx-auto mt-20 max-w-2xl text-center sm:mt-24">
          <p className="text-base/8 font-semibold text-cyan-600 dark:text-cyan-400">500</p>
          <h1 className="mt-4 text-5xl font-semibold tracking-tight text-balance text-gray-900 sm:text-6xl dark:text-white">
            An error occurred 🫠
          </h1>
          <p className="mt-6 text-lg font-medium text-pretty text-gray-500 sm:text-xl/8 dark:text-gray-400">
            Sorry, we couldn't find the page you're looking for.
          </p>
        </div>
        <div className="mx-auto mt-16 flow-root max-w-lg sm:mt-20">
          <h2 className="sr-only">Popular pages</h2>
          <ul className="-mt-6 divide-y divide-gray-900/5 border-b border-gray-900/5 dark:divide-white/10 dark:border-white/10">
            {links.map((link, linkIdx) => (
              <li key={linkIdx} className="relative flex gap-x-6 py-6">
                <div className="flex size-10 flex-none items-center justify-center rounded-lg shadow-xs outline-1 outline-gray-900/10 dark:bg-gray-800/50 dark:-outline-offset-1 dark:outline-white/10">
                  <link.icon
                    aria-hidden="true"
                    className="size-6 text-cyan-600 dark:text-cyan-400"
                  />
                </div>
                <div className="flex-auto">
                  <h3 className="text-sm/6 font-semibold text-gray-900 dark:text-white">
                    <a href={link.href}>
                      <span aria-hidden="true" className="absolute inset-0" />
                      {link.name}
                    </a>
                  </h3>
                  <p className="mt-2 text-sm/6 text-gray-600 dark:text-gray-400">
                    {link.description}
                  </p>
                </div>
                <div className="flex-none self-center">
                  <ChevronRightIcon
                    aria-hidden="true"
                    className="size-5 text-gray-400 dark:text-gray-500"
                  />
                </div>
              </li>
            ))}
          </ul>
          <div className="mt-10 flex justify-center">
            <a href="/" className="text-sm/6 font-semibold text-cyan-600 dark:text-cyan-400">
              <span aria-hidden="true">&larr;</span> Back to home
            </a>
          </div>
        </div>
      </main>
      <SiteFooter />
    </>
  );
}
