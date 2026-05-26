# walletui

`walletui` 是一个本地密码管理 TUI 工具。它提供密码的增删改查、分类、模糊搜索和批量导入能力，数据加密保存在本地 vault 文件中。

## 功能

- TUI 密码管理界面
- 新增、编辑、删除、查看密码
- 分类过滤
- 模糊搜索分类、名称、账号、备注
- 密码显示/隐藏切换
- 支持终端鼠标选中复制
- CSV / TSV 批量导入
- 本地加密 vault，不依赖系统钥匙串或外部密码管理软件

## 安装与运行

```bash
cargo run
```

启动后输入主密码。默认 vault 文件路径：

```text
~/.walletui.vault
```

也可以用环境变量指定主密码和 vault 路径：

```bash
WALLETUI_MASTER=your-master-password cargo run
WALLETUI_VAULT=/path/to/vault WALLETUI_MASTER=your-master-password cargo run
```

## TUI 快捷键

浏览模式：

| 快捷键 | 功能 |
| --- | --- |
| `↑` / `k` | 上一条 |
| `↓` / `j` | 下一条 |
| `a` | 新增密码 |
| `e` | 编辑当前密码 |
| `d` | 删除当前密码 |
| `/` | 模糊搜索 |
| `c` | 分类过滤 |
| `v` | 显示/隐藏密码 |
| `q` / `Esc` / `Ctrl-C` | 退出 |

编辑模式：

| 快捷键 | 功能 |
| --- | --- |
| `Tab` / `↓` | 下一个字段 |
| `Shift-Tab` / `↑` | 上一个字段 |
| `←` / `→` | 移动编辑光标 |
| `Home` / `End` | 跳到字段开头/末尾 |
| `Backspace` | 删除光标前字符 |
| `Enter` | 保存 |
| `Esc` | 取消 |

## 批量导入

默认导入 CSV：

```bash
cargo run -- import passwords.csv
```

导入 TSV：

```bash
cargo run -- import passwords.tsv --format tsv
```

覆盖当前 vault：

```bash
cargo run -- import passwords.csv --replace
```

导入文件列格式：

```csv
category,title,username,password,notes
work,GitHub,alice,"s,ecret","2fa enabled"
```

说明：

- 默认追加到现有 vault。
- `--replace` 会用导入内容覆盖当前 vault。
- 第一行可以是英文或中文表头。
- `notes` 可为空。
- `category` 为空时会使用 `默认`。

## 数据与安全

vault 文件默认保存在 `~/.walletui.vault`，可通过 `WALLETUI_VAULT` 覆盖。

加密实现位于 `src/crypto.rs` 和 `src/storage.rs`：

- PBKDF2-SHA256 从主密码派生密钥
- HMAC-SHA256 做完整性校验
- 每次保存生成新的 salt 和 nonce
- vault 内容不会明文保存密码

注意：这是项目内置的轻量加密实现，不依赖系统钥匙串。请妥善保管主密码；主密码丢失后无法恢复 vault 内容。

## 开发检查

```bash
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
```
