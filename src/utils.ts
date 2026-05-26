import type { TgMessageEntity } from "./model";

type Entity = typeof TgMessageEntity.Type;
type ActiveEntity = {
  start: number;
  entity: Entity;
};
type ResultEntity = {
  type: string;
  text: string;
};

export function parseEntities(
  text: string,
  entities: readonly Entity[],
): ResultEntity[] {
  const result: ResultEntity[] = [];

  if (entities.length === 0) {
    return result;
  }

  // Avoid unnecessary copy if already sorted
  const sorted = [...entities].sort((a, b) => a.offset - b.offset);

  let utf16Pos = 0;
  let next = 0;

  const active: ActiveEntity[] = [];

  for (let i = 0; i < text.length;) {
    const first = text.charCodeAt(i);

    // Fast surrogate pair detection
    const charLen =
      first >= 0xd800 && first <= 0xdbff && i + 1 < text.length ? 2 : 1;

    // Start entities
    while (next < sorted.length && sorted[next]?.offset === utf16Pos) {
      active.push({
        start: i,
        // biome-ignore lint/style/noNonNullAssertion: already checked
        entity: sorted[next]!,
      });

      next++;
    }

    utf16Pos += charLen;

    const end = i + charLen;

    // End entities
    for (let j = active.length - 1; j >= 0; j--) {
      if (!active[j]) {
        continue;
      }
      // biome-ignore lint/style/noNonNullAssertion: already checked
      const item = active[j]!;
      const entity = item.entity;

      if (entity.offset + entity.length === utf16Pos) {
        result.push({ type: entity.type, text: text.slice(item.start, end) });

        // O(1) removal
        if (active[active.length - 1]) {
          // biome-ignore lint/style/noNonNullAssertion: already checked
          active[j] = active[active.length - 1]!;
        }
        active.pop();
      }
    }

    i = end;
  }

  return result;
}
