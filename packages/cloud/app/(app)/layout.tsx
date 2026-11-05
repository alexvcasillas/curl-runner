import { AppShell } from '@/components/AppShell';
import { getInfo, getMe } from '@/lib/app-data';

export default async function AppLayout({ children }: { children: React.ReactNode }) {
  const [me, info] = await Promise.all([getMe(), getInfo()]);
  return (
    <AppShell me={me} info={{ billing: info.billing }}>
      {children}
    </AppShell>
  );
}
