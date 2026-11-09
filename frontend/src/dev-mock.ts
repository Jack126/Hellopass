// 仅开发预览用：在没有 Tauri 运行时（普通浏览器）注入一个内存版 __TAURI__.invoke，
// 便于查看界面。生产构建不会引用本文件。
type Entry = {
  id: number
  site_name: string
  site_url: string
  username: string
  encrypted_password: string
  encrypted_notes?: string
  password?: string
  notes?: string
  created_at: string
  updated_at: string
  last_accessed?: string
  access_count: number
  tags: string[]
  category: string
  priority: number
}

const now = new Date().toISOString()
const daysAgo = (n: number) => new Date(Date.now() - n * 86400000).toISOString()

let entries: Entry[] = [
  { id: 1, site_name: 'GitHub', site_url: 'https://github.com', username: 'alice@example.com', encrypted_password: '', password: 'Gh!7xQ2p#2026', notes: '工作账号，已开启两步验证', created_at: daysAgo(120), updated_at: daysAgo(3), last_accessed: daysAgo(1), access_count: 42, tags: ['工作', '开发'], category: 'work', priority: 2 },
  { id: 2, site_name: 'Gmail', site_url: 'https://mail.google.com', username: 'alice@gmail.com', encrypted_password: '', password: 'Mail@2026Pass', notes: '个人邮箱', created_at: daysAgo(300), updated_at: daysAgo(40), last_accessed: daysAgo(2), access_count: 18, tags: ['个人'], category: 'social', priority: 1 },
  { id: 3, site_name: '公司内网 VPN', site_url: 'https://vpn.company.com', username: 'alice.wang', encrypted_password: '', password: 'Vpn#Secure88', notes: '每月更换一次', created_at: daysAgo(200), updated_at: daysAgo(15), last_accessed: daysAgo(4), access_count: 9, tags: ['工作'], category: 'work', priority: 2 },
  { id: 4, site_name: '淘宝', site_url: 'https://www.taobao.com', username: 'alice_tb', encrypted_password: '', password: 'Tb@shopping1', notes: '', created_at: daysAgo(400), updated_at: daysAgo(95), last_accessed: daysAgo(10), access_count: 5, tags: ['购物'], category: 'shopping', priority: 0 },
  { id: 5, site_name: '招商银行', site_url: 'https://www.cmbchina.com', username: 'alice', encrypted_password: '', password: 'Bank!2026Safe', notes: '重要，勿泄露', created_at: daysAgo(500), updated_at: daysAgo(200), last_accessed: daysAgo(30), access_count: 3, tags: ['重要', '金融'], category: 'other', priority: 3 },
  { id: 6, site_name: 'Steam', site_url: 'https://store.steampowered.com', username: 'alice_gamer', encrypted_password: '', password: 'Gamer#Steam9', notes: '', created_at: daysAgo(150), updated_at: daysAgo(60), last_accessed: daysAgo(7), access_count: 26, tags: ['娱乐'], category: 'entertainment', priority: 0 },
]

const ok = (data: any) => ({ success: true, data, error: null, timestamp: new Date().toISOString() })
const fail = (error: string) => ({ success: false, data: null, error, timestamp: new Date().toISOString() })

function handle(cmd: string, args: any) {
  switch (cmd) {
    case 'is_master_password_set':
      return ok(true)
    case 'unlock':
      return ok(undefined)
    case 'setup_master_password':
      return ok(undefined)
    case 'lock':
      return ok(undefined)
    case 'change_master_password':
      return ok(undefined)
    case 'get_passwords':
      return ok(entries)
    case 'search_passwords': {
      const q = (args?.query?.query || '').toLowerCase()
      const list = entries.filter((e) => e.site_name.toLowerCase().includes(q) || e.username.toLowerCase().includes(q))
      return ok(list)
    }
    case 'add_password': {
      const p = args.password
      const e: Entry = {
        id: Date.now(),
        site_name: p.site_name,
        site_url: p.site_url,
        username: p.username,
        encrypted_password: '',
        password: p.password,
        notes: p.notes || '',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        access_count: 0,
        tags: p.tags || [],
        category: p.category || 'general',
        priority: p.priority || 0,
      }
      entries.unshift(e)
      return ok(e)
    }
    case 'update_password': {
      const p = args.password
      const idx = entries.findIndex((e) => e.id === p.id)
      if (idx < 0) return fail('未找到该密码')
      entries[idx] = { ...entries[idx], ...p, updated_at: new Date().toISOString() }
      return ok(entries[idx])
    }
    case 'delete_password':
      entries = entries.filter((e) => e.id !== args.id)
      return ok(undefined)
    case 'get_password_stats':
      return ok({ total_count: entries.length, category_counts: {}, tag_counts: {}, recent_access_count: 0 })
    case 'export_passwords':
      return ok(JSON.stringify(entries, null, 2))
    case 'export_passwords_csv':
      return ok('site_name,username')
    case 'export_selected': {
      const ids: number[] = args?.ids || []
      const sel = entries.filter((e) => ids.includes(e.id))
      if (args?.format === 'csv') return ok('site_name,username')
      return ok(JSON.stringify(sel, null, 2))
    }
    case 'import_passwords':
      return ok({ imported_count: 0, skipped_count: 0, updated_count: 0, error_count: 0, errors: [] })
    case 'export_backup':
      return ok(JSON.stringify({ version: '1.0', iterations: 600000, salt: '', data: '' }))
    case 'restore_backup':
      return ok({ imported_count: entries.length, skipped_count: 0, updated_count: 0, error_count: 0, errors: [] })
    case 'get_password_policy':
      return ok({ max_age_days: 90, history_limit: 5 })
    case 'get_expiring_passwords':
      return ok([
        { id: 5, site_name: '招商银行', days_until_expiry: -110, expired: true },
        { id: 4, site_name: '淘宝', days_until_expiry: -5, expired: true },
        { id: 6, site_name: 'Steam', days_until_expiry: 30, expired: false },
      ])
    case 'get_audit_logs':
      return ok([
        { id: 3, timestamp: daysAgo(1), action: 'unlock', resource_type: 'session', resource_id: null, details: null, success: true, error_message: null },
        { id: 2, timestamp: daysAgo(1), action: 'add_password', resource_type: 'password', resource_id: '1', details: null, success: true, error_message: null },
        { id: 1, timestamp: daysAgo(2), action: 'unlock', resource_type: 'session', resource_id: null, details: null, success: false, error_message: '主密码错误' },
      ])
    case 'check_password_strength':
      return ok({ is_valid: true, score: 100, feedback: [], suggestions: [] })
    case 'check_integrity':
      return ok({ sqlite_ok: true, db_checksum: 'a'.repeat(64), db_size: 40960, config_checksum: 'b'.repeat(64), baseline_ok: true })
    default:
      return ok(undefined)
  }
}

export function installMockTauri() {
  const w = window as any
  if (w.__TAURI__ && (w.__TAURI__.core?.invoke || w.__TAURI__.invoke)) return
  const mockInvoke = async (cmd: string, args?: any) => handle(cmd, args)
  w.__TAURI__ = {
    core: { invoke: mockInvoke },
    invoke: mockInvoke,
  }
}
