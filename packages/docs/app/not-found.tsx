import type { Metadata } from 'next';
import { StatusPage } from '@/components/site/StatusPage';

export const metadata: Metadata = {
  title: 'Page not found',
  description: 'The page you are looking for does not exist.',
};

export default function NotFound() {
  return (
    <StatusPage
      code="404"
      title="This page does not exist"
      description="The page you are looking for was moved or never existed."
    />
  );
}
