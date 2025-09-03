import { Github, Users } from 'lucide-react';
import Image from 'next/image';
import { Badge } from '@/components/ui/badge';

/**
 * X (formerly Twitter) logo SVG component
 */
const XIcon = ({ className }: { className?: string }) => (
  <svg fill="currentColor" viewBox="0 0 24 24" aria-hidden="true" className={className}>
    <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
  </svg>
);

/**
 * LinkedIn logo SVG component
 */
const LinkedInIcon = ({ className }: { className?: string }) => (
  <svg fill="currentColor" viewBox="0 0 24 24" aria-hidden="true" className={className}>
    <path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z" />
  </svg>
);

/**
 * Contributors data structure
 */
interface Contributor {
  name: string;
  role: string;
  imageUrl: string;
  githubUrl?: string;
  xUrl?: string;
  linkedinUrl?: string;
  isPlaceholder?: boolean;
}

/**
 * Contributors data - easily extensible for future contributors
 */
const contributors: Contributor[] = [
  {
    name: 'Alex Casillas',
    role: 'Creator & Maintainer',
    imageUrl: 'https://github.com/alexvcasillas.png',
    githubUrl: 'https://github.com/alexvcasillas',
    xUrl: 'https://x.com/alexvcasillas',
    linkedinUrl: 'https://linkedin.com/in/alexvcasillas',
  },
];

/**
 * Contributors section component that showcases project contributors
 * and encourages new contributors to join the project
 */
export function Contributors() {
  return (
    <section className="py-24 sm:py-32">
      <div className="mx-auto max-w-7xl px-6 lg:px-8">
        {/* Section Header */}
        <div className="mx-auto max-w-2xl lg:mx-0">
          <div className="flex items-center gap-2 mb-4">
            <Users className="h-6 w-6 text-cyan-500" />
            <Badge className="bg-cyan-500/10 text-cyan-600 dark:text-cyan-400 border-cyan-500/20">
              Community
            </Badge>
          </div>
          <h2 className="text-4xl font-semibold tracking-tight text-gray-900 sm:text-5xl dark:text-white">
            Contributors
          </h2>
          <p className="mt-6 text-lg text-gray-600 dark:text-gray-400">
            Meet the passionate developers who make curl-runner possible. Want to join us? We'd love
            to have you contribute to the project!
          </p>
        </div>

        {/* Contributors Grid */}
        <div className="mt-20 flex justify-start">
          {contributors.map((person) => (
            <div key={person.name} className="group">
              {/* Profile Image */}
              <div className="relative w-24 h-24">
                <Image
                  alt={person.name}
                  src={person.imageUrl}
                  width={96}
                  height={96}
                  className="w-full h-full rounded-full object-cover"
                />
                {/* Hover overlay for placeholder */}
                {person.isPlaceholder && (
                  <div className="absolute inset-0 rounded-full bg-gradient-to-br from-cyan-500/20 to-purple-500/20 opacity-0 group-hover:opacity-100 transition-opacity duration-300" />
                )}
              </div>

              {/* Contributor Info */}
              <h3 className="mt-6 text-lg font-semibold tracking-tight text-gray-900 dark:text-white">
                {person.name}
              </h3>
              <p className="text-base text-gray-600 dark:text-gray-400">{person.role}</p>

              {/* Social Links */}
              <div className="mt-6 flex gap-x-6">
                {person.githubUrl && (
                  <a
                    href={person.githubUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-gray-400 hover:text-gray-500 dark:text-gray-400 dark:hover:text-gray-200 transition-colors"
                  >
                    <span className="sr-only">
                      {person.isPlaceholder ? 'Contribute on GitHub' : 'GitHub'}
                    </span>
                    <Github className="h-5 w-5" />
                  </a>
                )}
                {person.xUrl && (
                  <a
                    href={person.xUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-gray-400 hover:text-gray-500 dark:text-gray-400 dark:hover:text-gray-200 transition-colors"
                  >
                    <span className="sr-only">X (Twitter)</span>
                    <XIcon className="h-5 w-5" />
                  </a>
                )}
                {person.linkedinUrl && (
                  <a
                    href={person.linkedinUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-gray-400 hover:text-gray-500 dark:text-gray-400 dark:hover:text-gray-200 transition-colors"
                  >
                    <span className="sr-only">LinkedIn</span>
                    <LinkedInIcon className="h-5 w-5" />
                  </a>
                )}
              </div>

              {/* Call-to-action for placeholder contributor */}
              {person.isPlaceholder && (
                <div className="mt-4">
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    Ready to contribute? Click to explore our repository and start your journey!
                  </p>
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
