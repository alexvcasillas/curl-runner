import { AnnouncementBanner } from '@/components/AnnouncementBanner';
import { AdminShell } from '@/components/admin/AdminShell';
import { getAdminMe } from '@/lib/admin-data';
import { getInfo, getMe } from '@/lib/app-data';

/**
 * The admin panel's own layout, outside the team shell. `getAdminMe` turns a
 * non-admin's 404 into the dashboard's 404 page.
 */
export default async function AdminLayout({ children }: { children: React.ReactNode }) {
  const [admin, me, info] = await Promise.all([getAdminMe(), getMe(), getInfo()]);
  return (
    <AdminShell
      me={me}
      admin={admin}
      version={info.version}
      announcement={<AnnouncementBanner banner={info.banner} />}
    >
      {children}
    </AdminShell>
  );
}
