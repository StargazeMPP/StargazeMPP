export const PROVIDER_CATEGORIES = [
  'on-chain-analytics',
  'physical-ai',
  'desci',
  'rwa',
  'compliance',
  'ai-model',
] as const;

export type ProviderCategory = (typeof PROVIDER_CATEGORIES)[number];

export function isProviderCategory(value: unknown): value is ProviderCategory {
  return typeof value === 'string' && (PROVIDER_CATEGORIES as readonly string[]).includes(value);
}
