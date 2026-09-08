/**
 * Author provenance classification (owner decision #293 follow-up):
 *
 *   'human' — written manually via a Corin/uteke UI surface.
 *   'agent' — everything else: agent-written, or legacy rows with no
 *             author stamp at all (assumed machine-written).
 *
 * The raw author string is deliberately not treated as a third taxonomy —
 * namespace already owns workspace grouping. Author only answers
 * "AI or human?".
 */
export type AuthorClass = 'human' | 'agent';

export function authorClass(
  m: { metadata?: Record<string, unknown> | null } | null | undefined,
): AuthorClass {
  return m?.metadata?.author === 'human' ? 'human' : 'agent';
}
