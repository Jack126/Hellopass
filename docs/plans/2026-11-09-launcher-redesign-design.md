# Hellopass 启动器化改造设计

日期：2026-11-09
状态：已实施。2026-11-09 调整：托盘导入/导出已移除，统一走设置窗（原生文件框+口令弹窗不可靠且难观察）。

## 1. 背景与目标

当前 Hellopass 是「900×700 主窗口 + 极简托盘」的形态：启动即渲染完整界面，未解锁时也挂着
密码列表、两个 el-dialog 和设置页；Element Plus 全量引入（1.09 MB JS + 366 KB CSS）并全局注册
300+ 图标。结果是内存占用高、启动重，且与「快速取一个密码」的主场景不匹配。

目标：改成启动器形态——常驻托盘，全局热键唤出一个极简搜索窗，输入即搜，回车复制密码并收起；
导入/导出/设置移到托盘，设置使用按需创建的独立窗口。

非目标：
- 不做跨设备同步。
- 不做 Windows Hello / 系统凭据解锁。
- 不做高级搜索语法（tag:、site: 等）。
- 不做全拼搜索，仅支持拼音首字母。

## 2. 交互与窗口

### 2.1 窗口

| label | 形态 | 创建时机 |
|---|---|---|
| launcher | 680×420，无边框、圆角、透明、置顶、skipTaskbar、visible=false | 随进程启动 |
| settings | 720×600，普通窗口 | 托盘点「设置」时创建，关闭即销毁 |

launcher 始终不占任务栏；关闭按钮不存在，Esc / 失焦 / 再次按热键收起。

### 2.2 状态机

```
locked ──输入主密码──▶ search ──→/Tab──▶ detail ──编辑──▶ edit
   ▲                    │                  │                │
   └────托盘锁定─────────┴──────Esc─────────┴──────Esc───────┘
```

- 首次运行（未设置主密码）：直接进 setup 态，设置完成即解锁。
- 已解锁时唤出：直接进 search 态，输入框自动聚焦。
- 输入为空：显示最近使用的 8 条。
- 结果最多 8 条，仅显示首字母色块、站点名、用户名、标签。
- Enter：复制密码，窗口立即收起，30 秒后清空剪贴板。
- Ctrl+Enter：复制用户名（同样 30 秒清理）。
- → / Tab：展开详情（网址、标签、备注；密码打码，Alt 按住才显示）。
- Ctrl+N：新建；Delete（详情态二次确认）：删除。
- Esc：逐级后退；search 且输入为空时收起窗口。
- 失焦自动收起；口令输入态与编辑态暂停自动收起（suppress_hide）。

### 2.3 会话

取消默认 30 秒空闲自动锁，改为解锁后保持到手动锁定或退出。
设置窗保留「自动锁时长」，0 表示不自动锁，默认 0。
锁定动作：前端清空 summary 缓存与详情明文，后端丢弃主密钥。

## 3. 数据与命令

### 3.1 新增/调整命令

- `list_entry_summaries() -> Vec<EntrySummary>`
  - 字段：id, site_name, site_url, username, tags, category, updated_at,
    last_accessed, access_count, pinyin_initials
  - 不包含 encrypted_password / password / notes / metadata / checksum
  - 后端生成拼音首字母，前端不用做转换
- `copy_secret(id, field)`：field = password | username
  - 后端解密 → 写剪贴板 → 记录 hash → 30 秒后仅当剪贴板仍是该内容时清空
  - 锁定态直接返回错误，不泄露任何信息
  - 密码字符串不进入 WebView、不写日志
- `reveal_password(id) -> String`：详情态显式查看，前端在收起/锁定时立即丢弃

### 3.2 数据流

解锁成功 → 拉一次 summaries 缓存到前端 → 按键本地过滤 → 复制时按 id 回后端解密。
新增/编辑/删除成功后只更新对应条目，不整表重拉。

## 4. 搜索

前端纯函数 `fuzzy.ts`，每次按键 O(N)，1000 条 < 1 ms，不做防抖。

评分优先级：
1. 前缀命中（含拼音首字母前缀）
2. 词边界命中（git → GitHub）
3. 子串命中
4. 子序列命中（ghb → GitHub），按命中紧凑度加分

同分时按 `last_accessed` + `access_count` 做时间衰减加权。
命中字符高亮。中文按原字子串匹配，拼音首字母作为额外候选串。

## 5. 托盘与导入导出

菜单：搜索（显示热键）、立即锁定、导入密码…、导出密码…、设置…、退出。

- 导入：原生文件框选文件。`.spb` 需要导入密码，此时 launcher 切到「口令输入态」收集；
  JSON/CSV 直接导入。完成后系统通知「导入 N 条，跳过 M 条重复」。
- 导出：原生保存框选路径 + 口令输入态设置导出密码，固定全量导出为 `.spb`。
- 文件框、通知、剪贴板、热键全部在 Rust 侧调用，前端不申请插件权限。

设置窗用 el-tabs 收拢：通用（热键录制、自动锁）、密码策略、导入导出、备份恢复、
安全概览、审计日志。关闭即销毁。

## 6. 安全

- 主密钥、剪贴板内容、详情明文在收起/锁定时清理。
- 复制路径不经过 JS；`reveal_password` 是唯一明文出口，须显式触发。
- 搜索词不落库、不写日志；审计只记「复制 / 查看 / 编辑」。
- 剪贴板清理前比对 hash，避免误删用户后来复制的内容。

## 7. 依赖与工程

- Rust：`tauri-plugin-global-shortcut`、`tauri-plugin-dialog`、`tauri-plugin-notification`、
  `tauri-plugin-clipboard-manager`、`pinyin`。
- Vite 多入口：`index.html`（launcher，手写 CSS，不引 Element Plus）、`setting.html`。
- 目录：`frontend/src/launcher/`、`frontend/src/settings/`、`frontend/src/shared/`。
- capabilities：`launcher`、`settings` 两个 label，仅 `core:default`。

## 8. 风险与处理

| 风险 | 处理 |
|---|---|
| 热键注册失败（被输入法/驱动占用） | 启动日志 + 托盘项显示「热键不可用」，可点击重试；设置窗可改键 |
| 透明窗在部分 Win11 环境渲染异常 | 检测失败时退化为不透明深色背景 |
| 失焦隐藏打断口令输入/编辑 | suppress_hide 标志由前端显式开关 |
| 托盘导入时文件框抢焦点导致窗口收起 | 进入导入流程前置 suppress_hide |

## 9. 实施步骤

1. 后端：EntrySummary（含拼音首字母）、copy_secret、reveal_password、托盘菜单、全局热键、窗口创建。
2. launcher：解锁/搜索/详情/编辑四态 + fuzzy.ts + 键盘交互。
3. 设置窗拆出（复用现有面板）+ 托盘导入导出。
4. 删除旧主界面，补测试，本地打包验证。

## 10. 测试

- Rust：summaries 不含明文；锁定态 copy_secret/reveal_password 报错；拼音首字母正确；剪贴板 hash 比对。
- 前端：fuzzy 排序与高亮用例；Esc 逐级退出；锁定后缓存清空。
- 保留并改造 `lock.spec.ts`、`clear-sensitive-guard.spec.ts`。
- 手工：热键唤出/收起、失焦、复制后 30 秒清理、托盘导入导出、设置窗开关。
