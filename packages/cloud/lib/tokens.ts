// The server mints `crsl_` share tokens and `cri_` invite tokens as a prefix plus
// 32 random bytes in base64url without padding: 43 characters.
const BODY = '[A-Za-z0-9_-]{43}';
const SHARE = new RegExp(`^crsl_${BODY}$`);
const INVITE = new RegExp(`^cri_${BODY}$`);

/** Whether `value` can be a share link token. A page checks this before it calls the API. */
export function isShareToken(value: string): boolean {
  return SHARE.test(value);
}

/** Whether `value` can be an invite token. */
export function isInviteToken(value: string): boolean {
  return INVITE.test(value);
}
