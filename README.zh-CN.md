[English](README.md) | [한국어](README.ko.md) | [日本語](README.ja.md) | **简体中文**

# face

BMO 的脸，`bmo-face`：机壳屏幕上的全屏显示、绘制和修正表情的编辑器、支撑这两者的服务器，以及读取简报表情的
`face/expression` 工具条目。它作为一个服务运行在 127.0.0.1:3217，位于 `/bmo-face/` 之后。它从 BMO 借用两样东西：
一是 Web 账户，编辑器用它登录，显示端也用它登录；二是监听流，每条简报都连同其情绪和 BMO 的在线状态通过它送达。

## 目录结构

| 目录 | 内容 |
|---|---|
| `core/` | 脸的核心，一个 Rust crate：表情文档的校验及从旧格式的迁移、眼睛集合与说话的嘴型集合、几何、混合以及固定时钟的驱动器；测试位于 `tests/`，其迁移的旧文档位于 `tests/fixtures/` |
| `client/` | 屏幕部分，一个独立的 Bun 项目：基于 `@nuxt/ui` 的 Nuxt 显示端和编辑器；界面上的每一处文字都是 `i18n/locales/` 下韩语、英语、日语和简体中文语言包中的键；`app/console/` 下的控制台包入口及 `scripts/` 下的构建；`vendor/` 下内置的 bmo-ui 发布版本，以及 `patches/` 下该层对所固定软件包的补丁；测试位于 `tests/` |
| `deploy/` | 部署时保留的用户单元 `bmo-face.service`、以及 `system/` 下由人工一次性安装的原生显示器系统单元和 PAM 服务 |
| `displayer/` | 基于核心的原生显示器 Rust crate：通过 logind 接管面板并用 CPU 绘制，接收表情服务器的文档和语音引擎的简报，播放声音并提供注视 socket；测试及其 fixture 位于 `tests/` |
| `docs/` | 规范文档：脸及其服务器（`face.md`）、绘制所依据的设计（`expressions.md`）、部署（`deploy.md`）和开发台（`development.md`） |
| `scripts/` | 开发台的命令（数据根目录的副本和开发台本身），以及核心的 WebAssembly 构建 |
| `server/` | 基于核心的服务器 Rust crate：表情文档的准备、编辑器会话、服务器路由、透传以及工具条目；测试位于 `tests/` |
| `src/` | 表情文档的 TypeScript 定义，屏幕以 `#faces` 导入；`wasm/` 下核心的 WebAssembly 构建及其类型化模块，屏幕以 `#core` 导入；以及开发副本的模块 |
| `tests/` | 表情文档定义、核心 WebAssembly 构建和开发副本的测试 |

根目录下有：以 `core/`、`displayer/` 和 `server/` 为成员的 Cargo workspace 清单及其 lockfile；包含 `src/`、
`scripts/` 和 `tests/` 的根 Bun 项目的清单、lockfile 和 `tsconfig.json`；`testbed.toml`；`install.toml`（在新机器上安装本单元所需的内容）；附带两个
schema 的 `tool.toml`；`console-manifest.json`，即控制台界面的声明，包含清单中由本服务决定的字段；`rustfmt.toml`；
`korean-ledger.json`；`LICENSE`；以及 `CLAUDE.md`。`docs/PERSONAL-DATA.md` 指定了检查
`tests/personal-data.test.ts`，测试套件会与其他测试一起运行它。

## 文档

| 文档 | 说明 |
|---|---|
| `docs/face.md` | 显示端何时显示什么、BMO 通过什么途径发送什么、屏幕如何登录、固定时钟和驱动器、表情文档以及路径的写法和混合、脸的核心及其接口、原生显示器、编辑器、服务器路由和控制台界面 |
| `docs/expressions.md` | 动画中所画的 BMO 的脸：逐帧测量的结果、颜色、形状的画法、眼睛集合、说话的嘴型集合、动画的运动方式，以及每个默认表情画出的样子 |
| `docs/deploy.md` | 一个提交如何上线：发布、单元、生成的屏幕和控制台包、工具条目和代理，以及原生显示器的系统单元及其一次性设置 |
| `docs/development.md` | 开发台：数据根目录的副本、基于它运行的服务器和屏幕、其后的 BMO 开发控制台，以及如何为其创建登录 |
| `docs/PERSONAL-DATA.md` | 任何文件都不得包含的个人数据、允许的值，以及将其拦截在外的模式检查（`tests/personal-data.test.ts`） |

## 构建与测试

```sh
bmo-testbed cargo build --workspace
bmo-testbed cargo test --workspace
bmo-testbed cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
bun run check:types
bun test ./tests
(cd client && bun run check)
```

Cargo 命令通过持有共享构建目录的 `bmo-testbed` 运行；直接使用 `cargo` 会把 workspace 从头编译到检出目录自己的
`target/` 中。crate 的格式化和 lint 无需声明单独的步骤：只要变更涉及某个 crate，testbed 的 Cargo workspace 检查就会
对所有成员执行它们，变更检查也会编译所涉及的 crate。屏幕在报告之前要先在开发台（`docs/development.md`）上确认能正常
工作。

## 版本

版本号属于整个套件，在构建时从 git 标签读取；Cargo workspace 的 package 和屏幕的清单中写的只是永不变动的占位版本。按照 bmo-chat 的
`docs/conventions.md` 「Versions」 所述，版本只在所有者宣布的发布时才变动；契约变更不改变版本，部署以提交来区分。

## 许可证

MIT，见 `LICENSE`。
