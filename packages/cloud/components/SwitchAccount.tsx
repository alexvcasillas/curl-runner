import { routes } from '@/lib/routes';
import { buttonClasses } from './button-styles';

/**
 * Signs out and comes back to sign in. A form, not a link: signing out changes
 * state, and `/sign-out` only takes a POST.
 */
export function SwitchAccount({
  returnTo,
  as = 'button',
}: {
  returnTo: string;
  as?: 'button' | 'link';
}) {
  return (
    <form action={`${routes.signOut}?return_to=${encodeURIComponent(returnTo)}`} method="post">
      <button
        type="submit"
        className={
          as === 'button'
            ? buttonClasses('secondary', 'md', 'h-10! w-full text-[14px]!')
            : 'cursor-pointer border-0 bg-transparent p-0 text-[12px] text-blue hover:text-blue-hover'
        }
      >
        Use another account
      </button>
    </form>
  );
}
