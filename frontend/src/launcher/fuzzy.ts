import type { Summary } from './ipc';

export interface Scored {
  item: Summary;
  score: number;
  ranges: [number, number][];
}

interface Match {
  score: number;
  ranges: [number, number][];
}

/** 字面前缀 / 词边界 / 子串 / 子序列四档打分。 */
function literal(q: string, raw: string): Match | null {
  const text = raw.toLowerCase();
  const idx = text.indexOf(q);
  if (idx === 0) {
    return { score: 1000 + Math.min(40, q.length * 8), ranges: [[0, q.length]] };
  }
  if (idx > 0) {
    const boundary = /[\s\-_./]/.test(text[idx - 1]);
    const base = boundary ? 820 : 620;
    return {
      score: base - Math.min(180, idx * 4) + Math.min(80, q.length * 6),
      ranges: [[idx, idx + q.length]],
    };
  }
  // 子序列：允许跳字母，但越紧凑分越高
  const hits: number[] = [];
  let from = 0;
  for (const ch of q) {
    const found = text.indexOf(ch, from);
    if (found < 0) return null;
    hits.push(found);
    from = found + 1;
  }
  const span = hits[hits.length - 1] - hits[0] + 1;
  const compact = q.length / span;
  return {
    score: 260 + compact * 140 + Math.min(60, q.length * 4) - Math.min(80, hits[0] * 2),
    ranges: hits.map((p) => [p, p + 1] as [number, number]),
  };
}

function recencyScore(item: Summary): number {
  const stamp = item.last_accessed || item.updated_at;
  const t = Date.parse(stamp.endsWith('Z') ? stamp : stamp.replace(' ', 'T') + 'Z');
  if (Number.isNaN(t)) return Math.min(20, item.access_count);
  const days = Math.max(0, (Date.now() - t) / 86_400_000);
  const freshness = Math.max(0, 60 - days);
  return Math.min(70, freshness + item.access_count * 2);
}

/** 合并重叠的高亮区间。 */
export function highlight(text: string, ranges: [number, number][]): { text: string; hit: boolean }[] {
  if (!ranges.length) return [{ text, hit: false }];
  const sorted = [...ranges].sort((a, b) => a[0] - b[0]);
  const merged: [number, number][] = [];
  for (const r of sorted) {
    const last = merged[merged.length - 1];
    if (last && r[0] <= last[1]) last[1] = Math.max(last[1], r[1]);
    else merged.push([r[0], r[1]]);
  }
  const out: { text: string; hit: boolean }[] = [];
  let cursor = 0;
  for (const [start, end] of merged) {
    if (start > cursor) out.push({ text: text.slice(cursor, start), hit: false });
    out.push({ text: text.slice(start, end), hit: true });
    cursor = end;
  }
  if (cursor < text.length) out.push({ text: text.slice(cursor), hit: false });
  return out;
}

/** 空查询按最近使用排序；有查询按分数排序，同分看最近使用。 */
export function search(items: Summary[], rawQuery: string): Scored[] {
  const q = rawQuery.trim().toLowerCase();
  if (!q) {
    return [...items]
      .sort((a, b) => recencyScore(b) - recencyScore(a))
      .slice(0, 8)
      .map((item) => ({ item, score: 0, ranges: [] }));
  }

  const fields: { text: string; weight: number }[] = [];
  const out: Scored[] = [];
  for (const item of items) {
    fields.length = 0;
    fields.push({ text: item.site_name, weight: 1 });
    fields.push({ text: item.username, weight: 0.75 });
    fields.push({ text: item.pinyin_initials, weight: 0.85 });
    fields.push({ text: item.tags.join(' '), weight: 0.6 });
    fields.push({ text: item.site_url, weight: 0.4 });

    let best = 0;
    let bestRanges: [number, number][] = [];
    let bestField = -1;
    fields.forEach((field, index) => {
      if (!field.text) return;
      const m = literal(q, field.text);
      if (!m) return;
      const score = m.score * field.weight;
      if (score > best) {
        best = score;
        bestRanges = m.ranges;
        bestField = index;
      }
    });

    if (best > 0) {
      out.push({
        item,
        score: best + recencyScore(item),
        // 只有站点名命中才做高亮，避免把用户名/拼音的位置画到站名上
        ranges: bestField === 0 ? bestRanges : [],
      });
    }
  }

  out.sort((a, b) => b.score - a.score || recencyScore(b.item) - recencyScore(a.item));
  return out.slice(0, 8);
}
