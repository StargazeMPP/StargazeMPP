/**
 * Privacy tier for a registered provider. Determines whether queries return
 * raw data (`open`), ZK-aggregated stats (`zk-aggregate`), confidential
 * payment-only protection (`confidential`), or per-buyer key-encrypted
 * raw data (`buyer-key`).
 *
 * Lowercase-with-hyphens — URL-safe and JSON-friendly. The backend column
 * `providers.privacy_tier` stores this exact string.
 */
export const PRIVACY_TIERS = [
  'open',
  'zk-aggregate',
  'confidential',
  'buyer-key',
] as const;

export type PrivacyTier = (typeof PRIVACY_TIERS)[number];

export function isPrivacyTier(value: unknown): value is PrivacyTier {
  return typeof value === 'string' && (PRIVACY_TIERS as readonly string[]).includes(value);
}
