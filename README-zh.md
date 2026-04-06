# WinSH - Windows Shell

[English](README.md) | 中文

用 Rust 编写的现代 Unix 风格 Windows 命令行 Shell。WinSH 在完整兼容 Windows 命令和 Unix 风格工具的同时，提供强大的 Shell 体验。

## 功能特性

### 核心功能
- **860+ 命令**：自动发现 PATH 中的命令
- **命令补全** : 通过配置文件自动构建命令补全机制
- **通配符展开**：完整支持 `*`、`?`、`[]` 模式
- **命令替换**：通过 `$(command)` 在命令中嵌套执行命令
- **脚本执行**：完整支持 `.sh` 脚本
- **历史记录**：方向键浏览命令历史

### 高级功能
- **数组系统**：定义、访问和操作数组
- **插件架构**：可扩展的插件系统
- **主题管理**：8 个内置主题，支持颜色自定义
- **环境变量**：完整的环境变量管理
- **Emacs 模式**：高效编辑的快捷键绑定

### 补全系统
- **带描述的参数补全**：Tab 补全时内联显示用法提示（如 `--regexp   A pattern to search for.`）
- **TOML 驱动定义**：从可配置目录中加载各命令的补全定义文件（详见[补全定义格式](#补全定义文件toml-格式)）
- **Bash 脚本自动导入**：启动时扫描补全目录中的 `_cmd.bash` / `cmd.bash`，自动解析为补全定义（详见[Bash 脚本导入](#bash-补全脚本自动导入)）
- **自动描述填充**：首次加载命令后自动运行 `cmd -h` 提取参数描述，后续从缓存直接读取（详见[自动描述填充](#自动描述填充cmd--h)）
- **环境变量补全**：输入 `$` 前缀时自动补全当前环境变量（详见[环境变量补全](#环境变量补全)）
- **三层缓存**：内存 → 磁盘（`.parsed.toml`）→ 子进程，基于 mtime 自动失效
- **多补全目录**：在 `~/.winshrc.toml` 中配置多个补全定义目录
- **列表弹出菜单**：浮动补全菜单，描述文字对齐显示

### 内置命令
| 命令 | 说明 |
|------|------|
| `ls` | 列出目录内容 |
| `cd` | 切换目录 |
| `pwd` | 显示当前目录 |
| `echo` | 输出文本 |
| `cat` | 显示文件内容 |
| `grep` | 文本搜索 |
| `find` | 查找文件 |
| `cp` | 复制文件 |
| `mv` | 移动/重命名文件 |
| `rm` | 删除文件 |
| `mkdir` | 创建目录 |
| `jobs` | 列出后台任务 |
| `fg` / `bg` | 前台/后台切换 |
| `set` / `unset` / `export` | 变量管理 |
| `alias` / `unalias` | 命令别名 |
| `array` | 数组操作 |
| `plugin` | 插件管理 |
| `theme` | 主题管理 |
| `history` | 命令历史 |
| `source` | 在当前 shell 执行脚本 |

## 安装

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/yourusername/winuxsh.git
cd winuxsh

# 构建发布版本
cargo build --release
```

### 配置 Windows Terminal

在 Windows Terminal 的 `settings.json` 中添加以下配置，将 `$env:PATH` 替换为实际路径：

```json
{
    "guid": "{9acb9455-ca63-5af2-ba0c-1fa3a891bd59}",
    "commandline": "${env:PATH}\\winuxsh.exe",
    "hidden": false,
    "name": "winuxsh"
}
```

## 配置

配置文件路径：`~/.winshrc.toml`

```toml
[shell]
prompt_format = "{user}@{host} {cwd} {symbol}"

[theme]
current_theme = "default"

[aliases]
ll = "ls -la"
la = "ls -a"

[completions]
# 可配置多个补全定义目录
completion_dirs = [
    "D:/shellTools/ripgrep/complete",
    "D:/shellTools/fd/autocomplete",
    "D:/shellTools/bat/autocomplete",
]
```

### 补全定义文件（TOML 格式）

在补全目录中创建 `<命令名>.toml`：

```toml
command = "mytool"
description = "My custom tool"

[[flags]]
short = "-v"
long = "--verbose"
description = "Enable verbose output"

[[flags]]
long = "--output"
description = "Output file path"
takes_value = true
values_from = "path"

[[flags]]
long = "--format"
description = "Output format"
takes_value = true
values = ["json", "yaml", "toml"]
```

### Bash 补全脚本自动导入

WinSH 启动时会扫描所有配置的补全目录，自动识别并解析其中的 bash 补全脚本（文件名以 `_` 开头，扩展名为 `.bash`，如 `_rg.bash`、`_fd.bash`）。

**工作流程：**

1. 扫描补全目录下的 `*.bash` 文件
2. 解析脚本中的 `opts="..."` 字段，提取所有短选项（`-x`）和长选项（`--xxx`）
3. 将解析结果序列化为 `~/.winsh/completions/cache/<cmd>.parsed.toml`（基于 bash 文件 mtime 自动失效）
4. 后续启动直接读取缓存，无需重新解析

**支持来源：** 大多数现代 CLI 工具（如 ripgrep、fd、bat、btm）在发布包中附带 `complete/` 或 `autocomplete/` 目录，里面包含 bash 补全脚本，直接指向这些目录即可。

> 如果目录中同时存在 `rg.toml` 和 `_rg.bash`，TOML 文件优先，bash 脚本被跳过。

### 自动描述填充（`cmd -h`）

bash 脚本本身不含参数描述文字，WinSH 会在首次加载命令后自动运行 `cmd -h` 补充描述：

**工作流程：**

1. 加载完所有补全定义后，对每个缺少描述的命令运行 `cmd -h`
2. 解析帮助文本，识别以下格式的参数行：
   ```
     -s, --case-sensitive             Description text
         --long-only                  Description text
     -e, --regexp=PATTERN             Description text
   ```
   以**两个或以上连续空格**作为参数名和描述文字的分隔符
3. 将提取到的描述写入对应 `FlagDef.description` 字段
4. **持久化回缓存**：将包含描述的完整定义覆写到 `.parsed.toml`，下次启动直接从缓存读取，不再重复运行 `cmd -h`

### 环境变量补全

输入 `$` 前缀后按 Tab，会自动补全当前 shell 的环境变量：

```bash
$ echo $PATH<Tab>
$ echo $HOME<Tab>
$ echo $USERPROFILE<Tab>

# 也支持部分匹配
$ echo $WIN<Tab>
# → $WINDIR, $WINUXSH_*, ...
```

**支持来源：** `export` / `set` 设置的变量、系统环境变量均可被补全。

## 使用示例

### 交互模式

```bash
./winuxsh.exe
```

### 执行单条命令

```bash
./winuxsh.exe -c "echo Hello World"
```

### 执行脚本

```bash
./winuxsh.exe script.sh
```

### 常用操作

```bash
# 通配符展开
ls *.rs
echo *.toml

# 命令替换
echo "当前用户: $(whoami)"

# 数组操作
array define colors red green blue
array get colors 0
array len colors

# 主题管理
theme list
theme set cyberpunk

# Tab 补全（带描述）
rg -<Tab>
# 显示:
# 0: --regexp       A pattern to search for.
# 1: --file         Search for patterns from the given file.
# 2: --after-context   Show NUM lines after each match.
# ...
```

## 项目结构

```
src/
├── main.rs               # 入口点与 REPL 循环
├── shell.rs              # Shell 状态与执行
├── tokenizer.rs          # 词法分析
├── parser.rs             # 语法分析
├── executor.rs           # 命令执行
├── builtins.rs           # 内置命令
├── array.rs              # 数组系统
├── plugin.rs             # 插件系统
├── theme.rs              # 主题管理
├── config.rs             # 配置管理
├── job.rs                # 任务控制
├── error.rs              # 错误处理
├── oh_my_winuxsh.rs      # Oh-My-Winuxsh 插件
└── completion/
    ├── mod.rs            # CompletionContext / CompletionResult
    ├── completer.rs      # WinuxshCompleter（reedline 集成）
    ├── external.rs       # 外部命令补全插件（TOML + bash + 缓存）
    ├── bash_import.rs    # Bash 补全脚本解析器
    ├── command.rs        # 命令名补全
    ├── path.rs           # 路径补全
    └── variables.rs      # 变量补全
```

## 主题系统

WinSH 内置 8 个主题：

| 主题名 | 风格描述 |
|--------|---------|
| `default` | 经典绿/蓝配色 |
| `dark` | 极简深色 |
| `light` | 浅色配色 |
| `colorful` | 鲜艳多彩 |
| `minimal` | 纯文本 |
| `cyberpunk` | 霓虹配色 |
| `ocean` | 蓝色调 |
| `forest` | 绿色调 |

## 性能

WinSH 通过 WinuxCmd DLL 集成实现命令智能路由，优化执行性能。

### 命令路由优先级

1. **内置命令** — 原生 WinSH 命令（最快）
2. **WinuxCmd DLL** — 通过 DLL 调用的 Unix 工具（极快）
3. **PATH 执行** — 外部可执行文件（标准性能）

### 性能基准（对比 WinuxCmd DLL vs PATH 执行）

| 场景 | WinuxCmd DLL | PATH 执行 | 提升 |
|------|-------------|----------|------|
| 单次执行（含启动开销） | 28.4ms | 55.3ms | 快 49% |
| 批量执行（10 条命令） | 4.6ms/条 | 31.7ms/条 | 快 ~7x |

## 兼容性

- **操作系统**：Windows 10 / 11
- **Rust**：2021 edition
- **推荐终端**：Windows Terminal
- **架构**：x64

## 开发

```bash
# Debug 构建
cargo build

# Release 构建
cargo build --release

# 运行测试
cargo test

# 格式化代码
cargo fmt

# Lint 检查
cargo clippy
```

## 贡献

欢迎贡献代码！请遵循以下流程：

1. Fork 仓库
2. 创建功能分支
3. 提交改动
4. 如有必要添加测试
5. 提交 Pull Request

## 开源许可

MIT License，详见 LICENSE 文件。

## 致谢

- **reedline**：Nushell 的行编辑库
- **winuxcmd**：Windows 上的 Unix 风格工具集
- **colored**：终端颜色支持

## 版本历史

### MVP6（当前）
- 数组支持
- 插件系统
- 主题管理
- 860+ 命令补全
- 完整通配符展开
- 命令替换
- 脚本执行
- TOML 驱动的外部命令补全
- Bash 补全脚本自动导入
- 从 `cmd -h` 提取参数描述并磁盘缓存
- 带对齐描述的 ListMenu 弹出补全菜单
- 多补全目录配置支持

### MVP5
- 任务控制
- 管道支持
- Vi 模式基础

### MVP4
- 基础 Shell 功能
