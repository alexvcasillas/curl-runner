import { ButtonLink } from '@/components/ButtonLink';
import { EmptyState } from '@/components/EmptyState';

export default function NotFound() {
  return (
    <main className="mx-auto box-border flex min-h-dvh w-full max-w-[560px] flex-col justify-center px-4 py-16">
      <EmptyState
        as="h1"
        code="404 Not Found"
        title="Nothing here"
        actions={<ButtonLink href="/">Go to your collections</ButtonLink>}
      >
        It doesn’t exist, or it belongs to a team you’re not in.
      </EmptyState>
    </main>
  );
}
