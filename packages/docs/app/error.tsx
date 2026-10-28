'use client';

import { StatusPage } from '@/components/site/StatusPage';

export default function ErrorPage(_props: { error: Error; reset: () => void }) {
  return (
    <StatusPage
      code="500"
      title="Something went wrong"
      description="An unexpected error occurred while loading this page. Try again in a moment."
    />
  );
}
