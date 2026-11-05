import 'server-only';
import { cache } from 'react';
import { api } from './api';
import { checked } from './collections-data';
import { ApiError } from './errors';
import type { BillingSummary, Prices, TeamDetail, Token } from './types';

/** `GET /v1/teams/{team}`: members and pending invites, once per request. */
export const getTeam = cache((team: string) => api<TeamDetail>(`/v1/teams/${checked(team)}`));

/** `GET …/tokens`: live tokens, newest first. No secrets. */
export const getTokens = cache((team: string) => api<Token[]>(`/v1/teams/${checked(team)}/tokens`));

/**
 * `GET /v1/billing/prices`, or null when it can't be read. Only call it with
 * billing on: with billing off the endpoint is a 404.
 */
export const getPrices = cache(async (): Promise<Prices | null> => {
  try {
    return await api<Prices>('/v1/billing/prices', { auth: 'none' });
  } catch (e) {
    if (e instanceof ApiError) {
      return null;
    }
    throw e;
  }
});

/** `GET …/billing`, or null when it can't be read. Billing on only. */
export const getBillingSummary = cache(async (team: string): Promise<BillingSummary | null> => {
  try {
    return await api<BillingSummary>(`/v1/teams/${checked(team)}/billing`);
  } catch (e) {
    if (e instanceof ApiError) {
      return null;
    }
    throw e;
  }
});
