# Prism

> 基于 TUI 的轻量级 Markdown 交互式演播工具

Prism 是一款运行在终端中的 Markdown 幻灯片放映工具。它将 Markdown 文件按 `---` 分隔符切分为多页幻灯片，解析行内格式（粗体、斜体、行内代码、链接、图片等），在终端中以全屏 TUI 模式进行渲染与交互式播放。

## 特性

- **Markdown 解析**：支持标题（H1-H6）、无序列表、代码块、段落、行内格式（粗体/斜体/行内代码/链接/图片）
- **幻灯片切分**：`---` 自动分页
- **演讲者模式**：`--presenter` 开启右侧小抄面板 + 实时计时器
- **演讲者备注**：`<!-- 备注内容 -->` HTML 注释自动提取为小抄
- **三套主题**：`default`（蓝黑）、`matrix`（绿黑）、`dracula`（暗紫）
- **页码跳转**：按 `G` 输入数字快速跳页
- **Unicode 框线代码块**：自适应宽度的 `┌─┐└─┘` 风格代码框
- **图片打开**：按 `O` 用系统查看器打开本地图片
- **纯文本提取 API**：泛型 `InlineSpanCollector` trait 支持结构化/纯文本双输出

## 安装

```bash
# 克隆仓库
git clone https://github.com/TezukaKeigo/prism.git
cd prism

# 编译（需要 Rust 1.85+）
cargo build --release

# 可执行文件位于
# target/release/prism (Linux/macOS)
# target\release\prism.exe (Windows)
```

## 使用方法

### 基本播放

```bash
cargo run -- test.md
# 或
./target/release/prism test.md
```

### 切换主题

```bash
cargo run -- test.md -t matrix     # 黑客帝国风格
cargo run -- test.md -t dracula    # 德古拉暗紫风格
```

### 演讲者模式

```bash
cargo run -- test.md --presenter
```

左侧 70% 为观众看到的幻灯片，右侧 30% 为演讲者小抄面板，包含实时计时器和当前页备注。

### 命令行参数

```
prism <FILE_PATH> [-t <THEME>] [--presenter]

参数：
  <FILE_PATH>     Markdown 文件路径（必填）
  -t, --theme     主题名称 [default: default] [可选: matrix, dracula]
  --presenter     开启演讲者模式
  -h, --help      显示帮助信息
  -V, --version   显示版本号
```

## 键盘快捷键

| 按键 | 功能 |
|------|------|
| `→` / 空格 | 下一页 |
| `←` | 上一页 |
| `G` | 进入页码跳转模式（输入数字 + Enter 确认，Esc 取消） |
| `O` | 打开当前页第一张图片（系统默认查看器） |
| `Q` / `Esc` | 退出播放 |

## Markdown 语法支持

### 幻灯片切分

用 `---`（单独一行，前后无其他内容）分隔幻灯片：

```markdown
# 第一页
内容……

---

# 第二页
内容……
```

### 标题

```markdown
# 一级标题
## 二级标题
### 三级标题
#### 四级标题
##### 五级标题
###### 六级标题
```

`#` 后必须有一个空格，否则降级为普通段落。

### 行内格式

```markdown
**粗体文本**
*斜体文本*
***粗斜体文本***
`行内代码`
[链接文字](https://example.com)
![图片替代文字](image.png)
\*转义星号\*
```

支持嵌套：`**加粗里有*斜体***`

### 无序列表

```markdown
- 列表项
* 列表项
```

### 代码块

````markdown
```rust
fn main() {
    println!("Hello, Prism!");
}
```
````

代码块使用 Unicode 框线字符渲染，自适应宽度。

### 演讲者备注

```markdown
<!-- 这是单行备注 -->

<!--
这是多行备注
第二行
第三行
-->
```

备注在普通模式不可见，仅在 `--presenter` 模式下显示在右侧小抄面板。

## 主题

| 名称 | 底色 | 标题色 | 边框色 | 预览 |
|------|------|--------|--------|------|
| `default` | 黑 | 青色加粗 | 蓝 | `cargo run -- test.md` |
| `matrix` | 黑 | 绿色加粗 | 绿 | `cargo run -- test.md -t matrix` |
| `dracula` | `#282A36` 暗紫 | 紫色加粗 | 灰蓝 | `cargo run -- test.md -t dracula` |

## 项目结构

```
prism/
├── Cargo.toml
├── README.md
├── test.md                 # 功能测试用幻灯片
├── testpic.png             # 测试用图片
└── src/
    ├── main.rs             # 入口 + 终端初始化 + 事件循环
    ├── config.rs           # clap 命令行参数解析
    ├── error.rs            # 自定义错误类型 PrismError
    ├── parser.rs           # Markdown 解析器（幻灯片切分 + 行内解析 + 泛型 trait）
    ├── theme.rs            # 三套主题配色方案
    └── ui.rs               # TUI 布局 + 幻灯片渲染 + 演讲者面板
```

## 依赖

| Crate | 版本 | 用途 |
|-------|------|------|
| [ratatui](https://crates.io/crates/ratatui) | 0.26 | TUI 渲染框架 |
| [crossterm](https://crates.io/crates/crossterm) | 0.27 | 终端控制（raw mode、事件） |
| [clap](https://crates.io/crates/clap) | 4.4 | 命令行参数解析 |
| [unicode-width](https://crates.io/crates/unicode-width) | 0.1 | Unicode 字符宽度计算 |

## 说明

- 本项目虽支持Linux系统，但开发环境为Windows系统，为避免未知bug，建议在**Windows**环境下运行
- 若项目不能正常运行，请检查是否已安装全部环境依赖
- 如对本项目运行有任何疑问，可**飞书**联系作者
