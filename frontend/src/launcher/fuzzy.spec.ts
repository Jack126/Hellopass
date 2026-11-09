import { describe, expect, it } from 'vitest';
import { highlight, search } from './fuzzy';
import type { Summary } from './ipc';

const make = (over: Partial<Summary>): Summary => ({
  id: 1,
  site_name: '',
  site_url: '',
  username: '',
  tags: [],
  category: 'general',
  updated_at: '2026-01-01T00:00:00Z',
  last_accessed: null,
  access_count: 0,
  pinyin_initials: '',
  ...over,
});

const items: Summary[] = [
  make({ id: 1, site_name: 'GitHub', username: 'alice', pinyin_initials: 'github alice' }),
  make({ id: 2, site_name: '微信', username: 'bob', pinyin_initials: 'wx bob', tags: ['聊天'] }),
  make({ id: 3, site_name: '支付宝', pinyin_initials: 'zfb', last_accessed: '2026-11-01T00:00:00Z' }),
  make({ id: 4, site_name: 'Google', pinyin_initials: 'google' }),
];

describe('fuzzy search', () => {
  it('prefix beats substring', () => {
    const r = search(items, 'git');
    expect(r[0].item.site_name).toBe('GitHub');
    expect(r[0].ranges[0]).toEqual([0, 3]);
  });

  it('matches pinyin initials', () => {
    expect(search(items, 'wx')[0].item.site_name).toBe('微信');
  });

  it('matches subsequence with gaps', () => {
    expect(search(items, 'ghb')[0].item.site_name).toBe('GitHub');
  });

  it('matches username and tags', () => {
    expect(search(items, 'alice')[0].item.id).toBe(1);
    expect(search(items, '聊天')[0].item.id).toBe(2);
  });

  it('returns recent items for empty query, max 8', () => {
    const many = Array.from({ length: 12 }, (_, i) =>
      make({ id: 100 + i, site_name: 'site' + i, last_accessed: `2026-11-0${(i % 9) + 1}T00:00:00Z` })
    );
    const r = search(many, '   ');
    expect(r).toHaveLength(8);
  });

  it('does not match unrelated query', () => {
    expect(search(items, 'zzzz')).toHaveLength(0);
  });

  it('highlights merged ranges', () => {
    expect(highlight('GitHub', [[0, 3]])).toEqual([
      { text: 'Git', hit: true },
      { text: 'Hub', hit: false },
    ]);
  });
});
