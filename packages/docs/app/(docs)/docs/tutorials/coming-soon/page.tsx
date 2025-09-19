import type { Metadata } from 'next';
import Link from 'next/link';
import { ArrowLeft, BookOpen, Clock, Lightbulb } from 'lucide-react';
import { DocsPageHeader } from '@/components/docs-page-header';
import { Button } from '@/components/ui/button';

export const metadata: Metadata = {
	title: 'Tutorial Coming Soon',
	description: 'This tutorial is currently in development. Check back soon for step-by-step guides on using curl-runner.',
	alternates: {
		canonical: 'https://www.curl-runner.com/docs/tutorials/coming-soon',
	},
};

export default function ComingSoonPage() {
	return (
		<main className="relative py-6 lg:py-8">
			<div className="mx-auto w-full min-w-0 max-w-3xl">
				<div className="text-center space-y-8">
					{/* Icon */}
					<div className="flex justify-center">
						<div className="rounded-full bg-blue-500/10 p-6">
							<BookOpen className="h-12 w-12 text-blue-600 dark:text-blue-400" />
						</div>
					</div>

					{/* Heading */}
					<div className="space-y-4">
						<h1 className="text-3xl font-bold tracking-tight">Tutorial Coming Soon</h1>
						<p className="text-xl text-muted-foreground max-w-2xl mx-auto">
							We're working hard to create comprehensive, step-by-step tutorials for curl-runner.
							This content will be available soon!
						</p>
					</div>

					{/* What's Coming */}
					<div className="rounded-lg border bg-card p-8 text-left max-w-2xl mx-auto">
						<div className="flex items-center gap-3 mb-6">
							<Lightbulb className="h-5 w-5 text-yellow-600 dark:text-yellow-400" />
							<h2 className="text-lg font-semibold">What's Coming</h2>
						</div>
						<div className="space-y-4">
							<div className="flex items-start gap-3">
								<div className="h-2 w-2 bg-primary rounded-full mt-2 flex-shrink-0" />
								<div>
									<h3 className="font-medium mb-1">Interactive Tutorials</h3>
									<p className="text-sm text-muted-foreground">
										Step-by-step guides with code examples you can copy and run
									</p>
								</div>
							</div>
							<div className="flex items-start gap-3">
								<div className="h-2 w-2 bg-primary rounded-full mt-2 flex-shrink-0" />
								<div>
									<h3 className="font-medium mb-1">Real-World Examples</h3>
									<p className="text-sm text-muted-foreground">
										Practical scenarios from API testing to CI/CD integration
									</p>
								</div>
							</div>
							<div className="flex items-start gap-3">
								<div className="h-2 w-2 bg-primary rounded-full mt-2 flex-shrink-0" />
								<div>
									<h3 className="font-medium mb-1">Video Walkthroughs</h3>
									<p className="text-sm text-muted-foreground">
										Visual guides to complement the written tutorials
									</p>
								</div>
							</div>
						</div>
					</div>

					{/* Timeline */}
					<div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
						<Clock className="h-4 w-4" />
						<span>Expected completion: Coming soon</span>
					</div>

					{/* Alternative Actions */}
					<div className="space-y-4">
						<p className="text-muted-foreground">
							In the meantime, check out these helpful resources:
						</p>

						<div className="flex flex-col sm:flex-row gap-4 justify-center">
							<Button asChild>
								<Link href="/docs/quick-start">
									Quick Start Guide
								</Link>
							</Button>
							<Button asChild variant="outline">
								<Link href="/docs/examples/basic">
									View Examples
								</Link>
							</Button>
							<Button asChild variant="outline">
								<Link href="/docs/use-cases">
									Use Cases
								</Link>
							</Button>
						</div>
					</div>

					{/* Back Link */}
					<div className="pt-8">
						<Button asChild variant="ghost">
							<Link href="/docs/tutorials" className="flex items-center gap-2">
								<ArrowLeft className="h-4 w-4" />
								Back to Tutorials
							</Link>
						</Button>
					</div>
				</div>
			</div>
		</main>
	);
}