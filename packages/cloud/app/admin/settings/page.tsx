import type { Metadata } from 'next';
import { SettingsView } from '@/components/admin/SettingsView';
import { getAdminMe, getAdmins, getSettings } from '@/lib/admin-data';
import { getInfo } from '@/lib/app-data';
import { saveBanner, savePlans, saveSignup, sendTestEmail, setPublishing } from './actions';

export const metadata: Metadata = { title: 'Server settings' };

export default async function AdminSettingsPage() {
  // The guard first: a non-admin gets a 404 before any request goes out.
  const me = await getAdminMe();
  const [settings, admins, info] = await Promise.all([getSettings(), getAdmins(), getInfo()]);
  return (
    <SettingsView
      settings={settings}
      env={{
        signIn: info.sign_in,
        billing: info.billing,
        owners: admins.filter((a) => a.role === 'owner').length,
      }}
      permissions={me.permissions}
      actions={{
        signup: saveSignup,
        plans: savePlans,
        banner: saveBanner,
        publishing: setPublishing,
        testEmail: sendTestEmail,
      }}
    />
  );
}
