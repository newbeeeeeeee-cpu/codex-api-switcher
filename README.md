# Codex API Switcher

`Codex API Switcher` 是一个基于 Tauri 2 的桌面工具，用来快速切换 Codex 正在使用的 API Key、provider 和 `base_url`。

它适合这些场景：

- 你有多个 OpenAI API Key，需要频繁切换账号
- 你会在 `OpenAI`、`custom` 等不同 provider 之间切换
- 你会在官方接口、自建中转、代理网关之间切换 `base_url`
- 你不想每次都手动去改 `~/.codex/config.toml` 和 `auth.json`

这个工具的原则很简单：

- 只改 Codex 真正用到的两个字段
- 不重写整份 `config.toml`
- 支持把多个账号保存到本地，随时切换

## 它会改什么

应用账号时，只会修改这三处：

1. `auth.json` 里的 `OPENAI_API_KEY`
2. `config.toml` 里的 `model_provider`
3. `config.toml` 里当前生效 provider 对应的 `base_url`

其他配置保持不动。

当前生效 provider 会按表单里的 `provider` 写入顶层 `model_provider = "..."`，并定位对应 section，比如：

- `model_provider = "OpenAI"` 时，修改 `[model_providers.OpenAI]` 下的 `base_url`
- `model_provider = "custom"` 时，修改 `[model_providers.custom]` 下的 `base_url`

如果当前 `config.toml` 结构和工具预期完全对不上，工具会直接生成一份最小可用模板，至少包含：

- `model_provider`
- `model`
- `[model_providers.<provider>]`
- `wire_api = "responses"`
- `requires_openai_auth = true`
- `base_url`

如果当前 `auth.json` 结构不对，工具也会回退为重建最小模板，只保留 `OPENAI_API_KEY`。

## 配置文件位置

默认会读取当前系统用户的 `.codex` 目录。

- macOS / Linux
  - `~/.codex/auth.json`
  - `~/.codex/config.toml`
- Windows
  - `%USERPROFILE%\.codex\auth.json`
  - `%USERPROFILE%\.codex\config.toml`

已保存账号列表会单独存到：

- macOS / Linux
  - `~/.codex/account-switcher/profiles.json`
- Windows
  - `%USERPROFILE%\.codex\account-switcher\profiles.json`

如果你在 Windows 上实际使用的是 WSL 里的 Codex，那么真正生效的可能是 WSL 用户目录下的 `~/.codex`，不是 Windows 的 `%USERPROFILE%\.codex`。

## 功能

- 读取当前生效的 API Key、provider 和 `base_url`
- 保存多个账号到本地列表
- 从列表中载入账号
- 一键应用到 Codex
- 删除本地保存的账号
- 支持 macOS / Linux / Windows 路径

## 使用方法

### 1. 启动应用

启动后，左侧会显示：

- 当前平台
- 当前 Codex 目录
- 账号存储文件路径

右侧会显示：

- 当前生效的 API Key
- 当前生效的 `provider`
- 当前生效的 `base_url`
- 正在使用的 `auth.json` / `config.toml` 路径

### 2. 新建一个账号

在右侧表单填写：

- 账号名称
- `Provider`
- `OpenAI API Key`
- `OpenAI base_url`

然后点击：

- `保存到列表`

这样这个账号就会保存在本地，后面可以反复使用。

### 3. 直接切换到当前表单内容

填写好 API Key 和 `base_url` 后，点击：

- `应用到 Codex`

应用成功后，工具会立即写回 Codex 配置文件。

### 4. 从已保存账号切换

左侧列表中每个账号有三个动作：

- `载入`
  - 把这个账号加载到右侧表单里，但不立即写入 Codex
- `应用`
  - 直接把这个账号写入 Codex
- `删除`
  - 删除本地保存的账号

删除是二次确认：

1. 第一次点击后按钮会变成 `确认删除`
2. 再点一次才会真正删除

## 开发运行

安装依赖：

```bash
npm install
```

启动开发模式：

```bash
cargo tauri dev
```

## 构建

前端构建：

```bash
npm run build
```

Rust 单测：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

当前平台桌面构建：

```bash
cargo tauri build --debug
```

Windows 原始 `.exe` 交叉构建：

```bash
cargo build --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-gnu
```

当前仓库已经验证过 Windows 目标可以产出原始 `.exe`。

## 说明

- 这个工具只负责切换 Codex 配置，不管理你的 OpenAI 账户本身
- 本地保存的账号列表里会包含 API Key，请注意机器权限和备份安全
- 如果现有配置结构无法可靠更新，工具会直接写入一份新的最小模板配置

## 仓库

- GitHub: `git@github.com:newbeeeeeeee-cpu/codex-api-switcher.git`
