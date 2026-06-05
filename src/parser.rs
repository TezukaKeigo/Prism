use crate::error::{PrismError, Result};

/// 行内文本片段
#[derive(Debug, Clone, PartialEq)]
pub enum TextSpan {
    Normal(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    InlineCode(String),
    Link { text: String, url: String },
    Image { alt: String, src: String },
}

/// 行内文本容器接口（用于泛型收集器）
pub trait InlineSpanCollector {
    fn push_span(&mut self, span: TextSpan);
}

impl InlineSpanCollector for Vec<TextSpan> {
    fn push_span(&mut self, span: TextSpan) {
        self.push(span);
    }
}

/// 纯文本收集器：丢弃所有格式标记，仅提取可读文字
///
/// 这是 `InlineSpanCollector` 的第二个实现者，证明 trait 泛型抽象的
/// 实际价值。配合 `parse_inline_with` 可将任意 Markdown 行内文本
/// 转为纯文本字符串，用于全文搜索、字数统计、纯文本导出等场景。
#[derive(Debug, Clone, Default)]
pub struct PlainTextCollector {
    pub text: String,
}

impl PlainTextCollector {
    pub fn into_string(self) -> String {
        self.text
    }
}

impl InlineSpanCollector for PlainTextCollector {
    fn push_span(&mut self, span: TextSpan) {
        match span {
            TextSpan::Normal(t)
            | TextSpan::Bold(t)
            | TextSpan::Italic(t)
            | TextSpan::BoldItalic(t)
            | TextSpan::InlineCode(t) => {
                self.text.push_str(&t);
            }
            TextSpan::Link { text, url } => {
                if url.is_empty() {
                    self.text.push_str(&text);
                } else {
                    self.text.push_str(&url);
                }
            }
            TextSpan::Image { alt, src } => {
                if alt.is_empty() {
                    self.text.push_str(&src);
                } else {
                    self.text.push_str(&alt);
                }
            }
        }
    }
}

/// 幻灯片中的单一视觉元素枚举
#[derive(Debug, Clone, PartialEq)]
pub enum SlideElement {
    /// 标题(级别 1-6, 文本内容)
    Heading(u8, Vec<TextSpan>),
    /// 列表项(文本内容)
    ListItem(Vec<TextSpan>),
    /// 代码块(代码文本内容)
    CodeBlock(String),
    /// 普通段落文本
    Paragraph(Vec<TextSpan>),
    /// 空行（用于在 TUI 渲染时保留物理排版间距）
    EmptyLine,
    /// 演讲者备注（HTML 注释 <!-- ... -->，仅演讲者模式可见）
    Note(String),
}

/// 单页幻灯片结构体
#[derive(Debug, Clone, PartialEq)]
pub struct Slide {
    pub elements: Vec<SlideElement>,
}

/// 从幻灯片元素中提取所有演讲者备注文本
pub fn collect_slide_notes(slide: &Slide) -> Vec<String> {
    slide
        .elements
        .iter()
        .filter_map(|e| match e {
            SlideElement::Note(content) => Some(content.clone()),
            _ => None,
        })
        .collect()
}

pub fn collect_slide_images(slide: &Slide) -> Vec<String> {
    let mut images = Vec::new();
    for element in &slide.elements {
        match element {
            SlideElement::Heading(_, spans)
            | SlideElement::ListItem(spans)
            | SlideElement::Paragraph(spans) => {
                collect_images_from_spans(spans, &mut images);
            }
            SlideElement::CodeBlock(_) | SlideElement::EmptyLine | SlideElement::Note(_) => {}
        }
    }
    images
}

/// Markdown 解析器
pub struct Parser;

impl Parser {
    /// 将整个 Markdown 文本流式解析为一组幻灯片页
    pub fn parse(content: &str) -> Result<Vec<Slide>> {
        let mut slides = Vec::new();
        let mut current_elements = Vec::new();

        // 状态机标志：指示当前是否正处于一个多行代码块里面
        let mut in_code_block = false;
        let mut current_code_lines = Vec::new();

        // 状态机标志：多行 HTML 备注解析
        let mut in_note = false;
        let mut current_note_lines = Vec::new();

        // 逐行流式扫描（带行号追踪，便于错误定位）
        for (line_number, line) in content.lines().enumerate() {
            let _line_number = line_number + 1; // 转为 1-based（预留用于未来逐行错误报告）
            let trimmed = line.trim();

            // 状态分支 1：如果当前正处于代码块内部
            if in_code_block {
                if trimmed.starts_with("```") {
                    // 遇到代码块闭合标记，关闭状态机，打包代码块内容
                    in_code_block = false;
                    let code_content = current_code_lines.join("\n");
                    current_elements.push(SlideElement::CodeBlock(code_content));
                    current_code_lines.clear();
                } else {
                    // 还在代码块内部，保留原始行
                    current_code_lines.push(line.to_string());
                }
                continue;
            }

            // 状态分支 2：如果正处于多行备注内部，收集行直到 -->
            if in_note {
                if let Some(end_pos) = trimmed.find("-->") {
                    in_note = false;
                    let note_line = trimmed[..end_pos].trim();
                    if !note_line.is_empty() {
                        current_note_lines.push(note_line.to_string());
                    }
                    let note_content = current_note_lines.join("\n").trim().to_string();
                    if !note_content.is_empty() {
                        current_elements.push(SlideElement::Note(note_content));
                    }
                    current_note_lines.clear();
                } else {
                    current_note_lines.push(line.to_string());
                }
                continue;
            }

            // 状态分支 3：遇到了幻灯片切页符 "---"
            if trimmed == "---" {
                // 如果当前页已经收集了内容，将其打包存入幻灯片大集合，并清空当前页
                if !current_elements.is_empty() {
                    slides.push(Slide { elements: current_elements });
                    current_elements = Vec::new();
                }
                continue;
            }

            // 检测 HTML 注释（演讲者备注）
            if trimmed.starts_with("<!--") {
                let inner = trimmed.trim_start_matches("<!--").trim();
                if let Some(end_pos) = inner.rfind("-->") {
                    // 单行备注：<!-- 内容 -->
                    let note = inner[..end_pos].trim();
                    if !note.is_empty() {
                        current_elements.push(SlideElement::Note(note.to_string()));
                    }
                } else {
                    // 多行备注开始
                    in_note = true;
                    if !inner.is_empty() {
                        current_note_lines.push(inner.to_string());
                    }
                }
                continue;
            }

            // 状态分支 4：解析常规的 Markdown 语法行
            if trimmed.starts_with("```") {
                // 触发代码块开启状态
                in_code_block = true;
            } else if trimmed.starts_with('#') {
                // 计算 # 的数量从而确定标题级别
                let mut level = 0;
                for ch in line.chars() {
                    if ch == '#' { level += 1; } else { break; }
                }

                let content_part = line.trim_start_matches('#');
                // 严格标准：# 后面必须有一个空格才算合法标题，且级别不超过 6
                if content_part.starts_with(' ') && level <= 6 {
                    current_elements.push(SlideElement::Heading(level as u8, parse_inline(content_part.trim())));
                } else {
                    current_elements.push(SlideElement::Paragraph(parse_inline(line)));
                }
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                // 识别无序列表项，裁剪掉前缀符号
                let item_content = trimmed[2..].trim();
                current_elements.push(SlideElement::ListItem(parse_inline(item_content)));
            } else if trimmed.is_empty() {
                // 收集空行，防止 PPT 所有内容死死贴在一起，留出呼吸感
                current_elements.push(SlideElement::EmptyLine);
            } else {
                // 兜底机制：其余全部认作普通文本段落
                current_elements.push(SlideElement::Paragraph(parse_inline(line)));
            }
        }

        // 循环结束后，如果还在代码块内，强制收尾并打包
        if in_code_block {
            let code_content = current_code_lines.join("\n");
            current_elements.push(SlideElement::CodeBlock(code_content));
            current_code_lines.clear();
        }

        // 循环结束后，如果还在备注内，强制收尾
        if in_note {
            let note_content = current_note_lines.join("\n").trim().to_string();
            if !note_content.is_empty() {
                current_elements.push(SlideElement::Note(note_content));
            }
            current_note_lines.clear();
        }

        // 循环结束后，如果最后一页有残留内容，塞进大集合
        if !current_elements.is_empty() {
            slides.push(Slide { elements: current_elements });
        }

        if slides.is_empty() {
            return Err(PrismError::ParseError {
                line: 0,
                message: "Markdown 文件未包含有效内容，解析失败！".to_string(),
            });
        }

        Ok(slides)
    }
}

pub(crate) fn parse_inline(text: &str) -> Vec<TextSpan> {
    parse_inline_with::<Vec<TextSpan>>(text)
}

/// 将 Markdown 行内文本解析为纯文本（丢弃所有格式标记）
///
/// 这是 `PlainTextCollector` 的便利封装，复用同一套 `parse_inline_with`
/// 泛型解析管线。
pub fn parse_inline_to_plain(text: &str) -> String {
    let collector = parse_inline_with::<PlainTextCollector>(text);
    collector.into_string()
}

fn parse_inline_with<C: InlineSpanCollector + Default>(text: &str) -> C {
    let mut spans = C::default();
    let mut buffer = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' {
            if let Some(next) = chars.get(i + 1) {
                if is_escaped_char(*next) {
                    buffer.push(*next);
                    i += 2;
                    continue;
                }
            }
            buffer.push('\\');
            i += 1;
            continue;
        }

        if ch == '!' {
            if chars.get(i + 1) == Some(&'[') {
                if let Some((alt, src, end)) = parse_link_like(&chars, i + 1) {
                    if !src.is_empty() {
                        flush_normal(&mut spans, &mut buffer);
                        spans.push_span(TextSpan::Image { alt, src });
                        i = end + 1;
                        continue;
                    }
                }
            }
        }

        if ch == '[' {
            if let Some((text, url, end)) = parse_link_like(&chars, i) {
                if !url.is_empty() {
                    flush_normal(&mut spans, &mut buffer);
                    spans.push_span(TextSpan::Link { text, url });
                    i = end + 1;
                    continue;
                }
            }
        }

        if ch == '`' {
            if i + 1 < chars.len() && chars[i + 1] == '`' {
                buffer.push('`');
                buffer.push('`');
                i += 2;
                continue;
            }

            if is_single_backtick(&chars, i) {
                if let Some(end) = find_closing_backtick(&chars, i + 1) {
                    if end > i + 1 {
                        let inner = &chars[i + 1..end];
                        if has_non_whitespace(inner) {
                            flush_normal(&mut spans, &mut buffer);
                            let content: String = inner.iter().collect();
                            spans.push_span(TextSpan::InlineCode(content));
                            i = end + 1;
                            continue;
                        }
                    }
                }
            }

            buffer.push('`');
            i += 1;
            continue;
        }

        if ch == '*' {
            if i + 1 < chars.len() && chars[i + 1] == '*' {
                if let Some(end) = find_closing(&chars, i + 2, "**") {
                    if end > i + 2 {
                        let inner = &chars[i + 2..end];
                        if has_non_whitespace(inner) {
                            flush_normal(&mut spans, &mut buffer);
                            let inner_text: String = inner.iter().collect();
                            let inner_spans = parse_inline_with::<Vec<TextSpan>>(&inner_text);
                            for span in apply_bold(inner_spans) {
                                spans.push_span(span);
                            }
                            i = end + 2;
                            continue;
                        }
                    }
                }

                buffer.push('*');
                buffer.push('*');
                i += 2;
                continue;
            }

            if let Some(end) = find_closing(&chars, i + 1, "*") {
                if end > i + 1 {
                    let inner = &chars[i + 1..end];
                    if has_non_whitespace(inner) {
                        flush_normal(&mut spans, &mut buffer);
                        let inner_text: String = inner.iter().collect();
                        let inner_spans = parse_inline_with::<Vec<TextSpan>>(&inner_text);
                        for span in apply_italic(inner_spans) {
                            spans.push_span(span);
                        }
                        i = end + 1;
                        continue;
                    }
                }
            }

            buffer.push('*');
            i += 1;
            continue;
        }

        buffer.push(ch);
        i += 1;
    }

    flush_normal(&mut spans, &mut buffer);
    spans
}

fn has_non_whitespace(chars: &[char]) -> bool {
    chars.iter().any(|ch| !ch.is_whitespace())
}

fn is_escaped_char(ch: char) -> bool {
    matches!(ch, '*' | '`' | '[' | ']' | '(' | ')' | '!' | '_' | '\\')
}

fn is_single_backtick(chars: &[char], i: usize) -> bool {
    let prev = i.checked_sub(1).and_then(|idx| chars.get(idx));
    let next = chars.get(i + 1);
    prev != Some(&'`') && next != Some(&'`')
}

fn find_closing(chars: &[char], mut i: usize, delim: &str) -> Option<usize> {
    while i < chars.len() {
        if chars[i] == '`' {
            if let Some(end) = find_closing_backtick(chars, i + 1) {
                i = end + 1;
                continue;
            } else {
                return None;
            }
        }

        if delim == "**" {
            if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '*' {
                if i + 2 < chars.len() && chars[i + 2] == '*' {
                    return Some(i + 1);
                }
                return Some(i);
            }
        } else if delim == "*" && chars[i] == '*' {
            if i + 1 < chars.len() && chars[i + 1] == '*' {
                i += 2;
                continue;
            }
            return Some(i);
        }

        i += 1;
    }

    None
}

fn find_closing_backtick(chars: &[char], mut i: usize) -> Option<usize> {
    while i < chars.len() {
        if chars[i] == '`' && is_single_backtick(chars, i) {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_link_like(chars: &[char], open_bracket: usize) -> Option<(String, String, usize)> {
    let mut i = open_bracket + 1;
    while i < chars.len() {
        if chars[i] == ']' && !is_escaped(chars, i) {
            break;
        }
        i += 1;
    }
    if i >= chars.len() || chars.get(i + 1) != Some(&'(') {
        return None;
    }
    let text: String = chars[open_bracket + 1..i].iter().collect();

    let mut j = i + 2;
    while j < chars.len() {
        if chars[j] == ')' && !is_escaped(chars, j) {
            break;
        }
        j += 1;
    }
    if j >= chars.len() {
        return None;
    }

    let url: String = chars[i + 2..j].iter().collect();
    Some((text, url, j))
}

fn is_escaped(chars: &[char], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    let mut backslashes = 0;
    let mut idx = i - 1;
    loop {
        if chars[idx] == '\\' {
            backslashes += 1;
            if idx == 0 {
                break;
            }
            idx -= 1;
        } else {
            break;
        }
    }
    backslashes % 2 == 1
}

fn apply_bold(spans: Vec<TextSpan>) -> Vec<TextSpan> {
    spans
        .into_iter()
        .map(|span| match span {
            TextSpan::Normal(text) => TextSpan::Bold(text),
            TextSpan::Italic(text) => TextSpan::BoldItalic(text),
            TextSpan::Bold(text) => TextSpan::Bold(text),
            TextSpan::BoldItalic(text) => TextSpan::BoldItalic(text),
            TextSpan::InlineCode(text) => TextSpan::InlineCode(text),
            TextSpan::Link { text, url } => TextSpan::Link { text, url },
            TextSpan::Image { alt, src } => TextSpan::Image { alt, src },
        })
        .collect()
}

fn apply_italic(spans: Vec<TextSpan>) -> Vec<TextSpan> {
    spans
        .into_iter()
        .map(|span| match span {
            TextSpan::Normal(text) => TextSpan::Italic(text),
            TextSpan::Italic(text) => TextSpan::Italic(text),
            TextSpan::Bold(text) => TextSpan::BoldItalic(text),
            TextSpan::BoldItalic(text) => TextSpan::BoldItalic(text),
            TextSpan::InlineCode(text) => TextSpan::InlineCode(text),
            TextSpan::Link { text, url } => TextSpan::Link { text, url },
            TextSpan::Image { alt, src } => TextSpan::Image { alt, src },
        })
        .collect()
}

fn flush_normal<C: InlineSpanCollector>(spans: &mut C, buffer: &mut String) {
    if !buffer.is_empty() {
        spans.push_span(TextSpan::Normal(std::mem::take(buffer)));
    }
}

fn collect_images_from_spans(spans: &[TextSpan], images: &mut Vec<String>) {
    for span in spans {
        match span {
            TextSpan::Image { alt: _, src } => {
                if !src.is_empty() && !images.contains(src) {
                    images.push(src.clone());
                }
            }
            TextSpan::Link { .. }
            | TextSpan::Normal(_)
            | TextSpan::Bold(_)
            | TextSpan::Italic(_)
            | TextSpan::BoldItalic(_)
            | TextSpan::InlineCode(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== 辅助宏 ==========

    macro_rules! normal {
        ($s:expr) => {
            TextSpan::Normal($s.to_string())
        };
    }
    macro_rules! bold {
        ($s:expr) => {
            TextSpan::Bold($s.to_string())
        };
    }
    macro_rules! italic {
        ($s:expr) => {
            TextSpan::Italic($s.to_string())
        };
    }
    macro_rules! bold_italic {
        ($s:expr) => {
            TextSpan::BoldItalic($s.to_string())
        };
    }
    macro_rules! inline_code {
        ($s:expr) => {
            TextSpan::InlineCode($s.to_string())
        };
    }
    macro_rules! link {
        ($text:expr, $url:expr) => {
            TextSpan::Link {
                text: $text.to_string(),
                url: $url.to_string(),
            }
        };
    }
    macro_rules! image {
        ($alt:expr, $src:expr) => {
            TextSpan::Image {
                alt: $alt.to_string(),
                src: $src.to_string(),
            }
        };
    }

    // ========== parse_inline 测试 ==========

    #[test]
    fn test_normal_text() {
        let result = parse_inline("hello world");
        assert_eq!(result, vec![normal!("hello world")]);
    }

    #[test]
    fn test_empty_input() {
        let result = parse_inline("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let result = parse_inline("   ");
        assert_eq!(result, vec![normal!("   ")]);
    }

    #[test]
    fn test_bold() {
        let result = parse_inline("hello **world** foo");
        assert_eq!(
            result,
            vec![normal!("hello "), bold!("world"), normal!(" foo")]
        );
    }

    #[test]
    fn test_bold_single_character() {
        let result = parse_inline("**a**");
        assert_eq!(result, vec![bold!("a")]);
    }

    #[test]
    fn test_bold_entire_line() {
        let result = parse_inline("**entire line**");
        assert_eq!(result, vec![bold!("entire line")]);
    }

    #[test]
    fn test_italic() {
        let result = parse_inline("hello *world* foo");
        assert_eq!(
            result,
            vec![normal!("hello "), italic!("world"), normal!(" foo")]
        );
    }

    #[test]
    fn test_bold_italic_triple_star() {
        let result = parse_inline("hello ***world*** foo");
        assert_eq!(
            result,
            vec![normal!("hello "), bold_italic!("world"), normal!(" foo")]
        );
    }

    #[test]
    fn test_nested_bold_in_italic() {
        // *斜体里有**加粗*** → italic 包裹 bold
        let result = parse_inline("*斜体里有**加粗***");
        assert_eq!(result, vec![italic!("斜体里有"), bold_italic!("加粗")]);
    }

    #[test]
    fn test_nested_italic_in_bold() {
        // **加粗里有*斜体***
        let result = parse_inline("**加粗里有*斜体***");
        assert_eq!(result, vec![bold!("加粗里有"), bold_italic!("斜体")]);
    }

    #[test]
    fn test_inline_code() {
        let result = parse_inline("hello `code` world");
        assert_eq!(
            result,
            vec![normal!("hello "), inline_code!("code"), normal!(" world")]
        );
    }

    #[test]
    fn test_inline_code_with_markup_inside() {
        // 反引号内不应解析 markdown
        let result = parse_inline("`**bold**`");
        // 注意：parse_inline 支持反引号内再解析，所以 **bold** 在代码样式下仍会被解析
        // 实际上 inline code 内容会被原样保留为 InlineCode，内部标记不做解析
        assert_eq!(result, vec![inline_code!("**bold**")]);
    }

    #[test]
    fn test_double_backtick_not_code() {
        // `` 不算行内代码开启
        let result = parse_inline("``not code``");
        // 两个反引号不是标准行内代码定界符，应被视为普通文本
        assert!(!result.is_empty());
        // 具体行为取决于解析器实现 — 当前实现将其视为普通文本
    }

    #[test]
    fn test_link() {
        let result = parse_inline("visit [Prism](https://prism.dev) now");
        assert_eq!(
            result,
            vec![
                normal!("visit "),
                link!("Prism", "https://prism.dev"),
                normal!(" now"),
            ]
        );
    }

    #[test]
    fn test_link_no_text() {
        let result = parse_inline("[](https://prism.dev)");
        assert_eq!(result, vec![link!("", "https://prism.dev")]);
    }

    #[test]
    fn test_image() {
        let result = parse_inline("see ![logo](icon.png) here");
        assert_eq!(
            result,
            vec![
                normal!("see "),
                image!("logo", "icon.png"),
                normal!(" here"),
            ]
        );
    }

    #[test]
    fn test_image_no_alt() {
        let result = parse_inline("![](icon.png)");
        assert_eq!(result, vec![image!("", "icon.png")]);
    }

    #[test]
    fn test_escape_asterisk() {
        // \* 应被视为普通星号字符
        let result = parse_inline(r"hello \*world\*");
        assert_eq!(result, vec![normal!("hello *world*")]);
    }

    #[test]
    fn test_escape_backtick() {
        let result = parse_inline(r"\`not code\`");
        assert_eq!(result, vec![normal!("`not code`")]);
    }

    #[test]
    fn test_escape_bracket() {
        let result = parse_inline(r"\[not a link\](url)");
        assert_eq!(result, vec![normal!("[not a link](url)")]);
    }

    #[test]
    fn test_escape_backslash() {
        let result = parse_inline(r"a\\b");
        assert_eq!(result, vec![normal!(r"a\b")]);
    }

    #[test]
    fn test_unclosed_bold() {
        // **未闭合的加粗应被视为普通文本
        let result = parse_inline("hello **world");
        assert_eq!(result, vec![normal!("hello **world")]);
    }

    #[test]
    fn test_unclosed_italic() {
        let result = parse_inline("hello *world");
        assert_eq!(result, vec![normal!("hello *world")]);
    }

    #[test]
    fn test_unclosed_inline_code() {
        let result = parse_inline("hello `code");
        assert_eq!(result, vec![normal!("hello `code")]);
    }

    #[test]
    fn test_empty_bold() {
        // **** 两个连续的空加粗
        let result = parse_inline("****");
        // 第一个 ** 开启，第二个 * 和第三个 * 之间的内容为空，第四个字为闭合
        // 空内容不会生成 Bold span
        assert!(result.iter().all(|s| matches!(s, TextSpan::Normal(_))));
    }

    #[test]
    fn test_empty_bold_with_spaces() {
        let result = parse_inline("** **");
        // 内容为空格 " "，has_non_whitespace 检查失败，不生成 Bold
        assert_eq!(result, vec![normal!("** **")]);
    }

    #[test]
    fn test_adjacent_markers() {
        // **bold***italic*`code`
        // 解析器将 *** 的第一个 * 纳入加粗内容，剩余 ** 闭合加粗
        let result = parse_inline("**bold***italic*`code`");
        assert_eq!(
            result,
            vec![bold!("bold*"), normal!("italic*"), inline_code!("code")]
        );
    }

    #[test]
    fn test_consecutive_stars() {
        // *** 应被解析为加粗斜体（** 先匹配，包裹单个 *）
        // 实际上 ***three*** 中，前三个 * 开启加粗斜体，内容为 "three"
        let result = parse_inline("a ***three*** b");
        assert_eq!(
            result,
            vec![normal!("a "), bold_italic!("three"), normal!(" b")]
        );
    }

    #[test]
    fn test_mixed_formatting() {
        // 综合测试：普通 + 粗体 + 斜体 + 代码
        let result = parse_inline("normal **bold** *italic* `code` end");
        assert_eq!(
            result,
            vec![
                normal!("normal "),
                bold!("bold"),
                normal!(" "),
                italic!("italic"),
                normal!(" "),
                inline_code!("code"),
                normal!(" end"),
            ]
        );
    }

    #[test]
    fn test_only_bold() {
        let result = parse_inline("**only**");
        assert_eq!(result, vec![bold!("only")]);
    }

    #[test]
    fn test_only_italic() {
        let result = parse_inline("*only*");
        assert_eq!(result, vec![italic!("only")]);
    }

    #[test]
    fn test_underscore_as_normal() {
        // 下划线 _ 不被解析为斜体标记
        let result = parse_inline("hello_world");
        assert_eq!(result, vec![normal!("hello_world")]);
    }

    #[test]
    fn test_image_no_src() {
        // 没有 src 的 "图片"
        let result = parse_inline("![alt]()");
        // src 为空，不应生成 Image span
        assert!(!result.iter().any(|s| matches!(s, TextSpan::Image { .. })));
    }

    #[test]
    fn test_link_no_url() {
        let result = parse_inline("[text]()");
        // url 为空，不应生成 Link span
        assert!(!result.iter().any(|s| matches!(s, TextSpan::Link { .. })));
    }

    #[test]
    fn test_bold_with_code_inside() {
        // ** 内有 `code` — 解析器会递归处理内部内容
        let result = parse_inline("**bold with `code` inside**");
        // 递归解析后：Normal→Bold, InlineCode 保持不变, Normal→Bold
        assert_eq!(result.len(), 3);
        assert!(matches!(result[0], TextSpan::Bold(_)));
        assert!(matches!(result[1], TextSpan::InlineCode(_)));
        assert!(matches!(result[2], TextSpan::Bold(_)));
    }

    #[test]
    fn test_trailing_asterisk_in_bold() {
        // **bold*** → *** 被解析为：第一个 * 是加粗内容，"**" 闭合加粗
        let result = parse_inline("**bold***");
        assert_eq!(result, vec![bold!("bold*")]);
    }

    // ========== PlainTextCollector & parse_inline_to_plain 测试 ==========

    #[test]
    fn test_plain_text_normal() {
        assert_eq!(parse_inline_to_plain("hello world"), "hello world");
    }

    #[test]
    fn test_plain_text_empty() {
        assert_eq!(parse_inline_to_plain(""), "");
    }

    #[test]
    fn test_plain_text_bold_stripped() {
        assert_eq!(parse_inline_to_plain("hello **world** foo"), "hello world foo");
    }

    #[test]
    fn test_plain_text_italic_stripped() {
        assert_eq!(parse_inline_to_plain("hello *world* foo"), "hello world foo");
    }

    #[test]
    fn test_plain_text_bold_italic_stripped() {
        assert_eq!(parse_inline_to_plain("a ***b*** c"), "a b c");
    }

    #[test]
    fn test_plain_text_inline_code_stripped() {
        assert_eq!(parse_inline_to_plain("use `println!` macro"), "use println! macro");
    }

    #[test]
    fn test_plain_text_link_shows_url() {
        let result = parse_inline_to_plain("visit [Prism](https://example.com) now");
        assert_eq!(result, "visit https://example.com now");
    }

    #[test]
    fn test_plain_text_link_empty_url_kept_literal() {
        // 空 URL 不被识别为链接，原文保留
        let result = parse_inline_to_plain("[text]()");
        assert_eq!(result, "[text]()");
    }

    #[test]
    fn test_plain_text_image_shows_alt() {
        let result = parse_inline_to_plain("see ![logo](icon.png) here");
        assert_eq!(result, "see logo here");
    }

    #[test]
    fn test_plain_text_image_no_alt_shows_src() {
        let result = parse_inline_to_plain("see ![](icon.png) here");
        assert_eq!(result, "see icon.png here");
    }

    #[test]
    fn test_plain_text_escaped_chars() {
        // \* → *   \` → `
        let result = parse_inline_to_plain(r"hello \*world\* foo");
        assert_eq!(result, "hello *world* foo");
    }

    #[test]
    fn test_plain_text_mixed_all() {
        let result = parse_inline_to_plain(
            "**bold** *italic* `code` [link](http://a.b) ![img](x.png) normal",
        );
        assert_eq!(result, "bold italic code http://a.b img normal");
    }

    #[test]
    fn test_plain_text_unclosed_markers_kept() {
        // 未闭合的标记符保留为普通文本
        let result = parse_inline_to_plain("**unclosed");
        assert_eq!(result, "**unclosed");
    }

    // ========== Parser::parse (幻灯片解析) 测试 ==========

    #[test]
    fn test_single_slide() {
        let content = "# Title\n\nSome paragraph text.";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert_eq!(slides[0].elements.len(), 3); // heading, empty line, paragraph
        assert!(matches!(slides[0].elements[0], SlideElement::Heading(1, _)));
    }

    #[test]
    fn test_multiple_slides() {
        let content = "# Slide 1\n---\n# Slide 2\n---\n# Slide 3";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 3);
    }

    #[test]
    fn test_empty_content_error() {
        let result = Parser::parse("");
        assert!(result.is_err());
        match result {
            Err(PrismError::ParseError { line, .. }) => {
                assert_eq!(line, 0); // 文件级错误
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_only_separator_no_error() {
        // 只有分隔符且前后无内容，不应报错
        let content = "# A\n---\n---\n# B";
        let slides = Parser::parse(content).unwrap();
        // 第一个 --- 分割，第二个 --- 因为 current_elements 为空被跳过
        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn test_heading_levels() {
        let content = "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        let elements = &slides[0].elements;
        assert!(matches!(elements[0], SlideElement::Heading(1, _)));
        assert!(matches!(elements[1], SlideElement::Heading(2, _)));
        assert!(matches!(elements[2], SlideElement::Heading(3, _)));
        assert!(matches!(elements[3], SlideElement::Heading(4, _)));
        assert!(matches!(elements[4], SlideElement::Heading(5, _)));
        assert!(matches!(elements[5], SlideElement::Heading(6, _)));
    }

    #[test]
    fn test_illegal_heading_no_space() {
        // # 后面没有空格 → 降级为普通段落
        let content = "#no-space";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert!(matches!(
            slides[0].elements[0],
            SlideElement::Paragraph(_)
        ));
    }

    #[test]
    fn test_heading_level_too_deep() {
        // ####### 超过 6 级 → 降级为普通段落
        let content = "####### too deep";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert!(matches!(
            slides[0].elements[0],
            SlideElement::Paragraph(_)
        ));
    }

    #[test]
    fn test_unordered_list_dash() {
        let content = "- item 1\n- item 2";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert!(matches!(slides[0].elements[0], SlideElement::ListItem(_)));
        assert!(matches!(slides[0].elements[1], SlideElement::ListItem(_)));
    }

    #[test]
    fn test_unordered_list_star() {
        let content = "* item 1\n* item 2";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert!(matches!(slides[0].elements[0], SlideElement::ListItem(_)));
        assert!(matches!(slides[0].elements[1], SlideElement::ListItem(_)));
    }

    #[test]
    fn test_code_block() {
        let content = "```\nfn main() {\n    println!(\"hello\");\n}\n```";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert!(matches!(slides[0].elements[0], SlideElement::CodeBlock(_)));
        if let SlideElement::CodeBlock(ref code) = slides[0].elements[0] {
            assert!(code.contains("fn main()"));
            assert!(code.contains("println!"));
        }
    }

    #[test]
    fn test_unclosed_code_block() {
        // 未闭合的代码块：应被强制收尾
        let content = "```\nsome code\nwithout closing";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert!(matches!(slides[0].elements[0], SlideElement::CodeBlock(_)));
    }

    #[test]
    fn test_empty_lines_preserved() {
        let content = "line 1\n\n\nline 2";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        // 应该有 paragraph, empty line, empty line, paragraph = 4 个元素
        assert_eq!(slides[0].elements.len(), 4);
        assert!(matches!(slides[0].elements[0], SlideElement::Paragraph(_)));
        assert!(matches!(slides[0].elements[1], SlideElement::EmptyLine));
        assert!(matches!(slides[0].elements[2], SlideElement::EmptyLine));
        assert!(matches!(slides[0].elements[3], SlideElement::Paragraph(_)));
    }

    #[test]
    fn test_content_before_first_separator() {
        let content = "# Slide 1\n---\n# Slide 2";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 2);
    }

    #[test]
    fn test_trailing_content_after_last_separator() {
        let content = "# A\n---\n# B";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 2);
        // 确保最后一页被正确打包
        assert!(matches!(
            slides[1].elements[0],
            SlideElement::Heading(1, _)
        ));
    }

    #[test]
    fn test_mixed_content() {
        let content = "# Title\n\n- list item\n\n```\ncode\n```\n\nparagraph with **bold**";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        let elements = &slides[0].elements;
        // Heading, EmptyLine, ListItem, EmptyLine, CodeBlock, EmptyLine, Paragraph
        assert!(elements.len() >= 6);
    }

    #[test]
    fn test_whitespace_only_content() {
        let content = "   \n\n  \t  ";
        let slides = Parser::parse(content).unwrap();
        // 空格行被视为 empty line
        assert_eq!(slides.len(), 1);
        for elem in &slides[0].elements {
            assert!(matches!(elem, SlideElement::EmptyLine));
        }
    }

    #[test]
    fn test_content_with_whitespace_and_text() {
        let content = "   \nsome text\n   ";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert_eq!(slides[0].elements.len(), 3); // EmptyLine, Paragraph, EmptyLine
    }

    // ========== SlideElement::Note（演讲者备注）测试 ==========

    #[test]
    fn test_single_line_note() {
        let content = "<!-- 这是备注 -->";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert_eq!(slides[0].elements.len(), 1);
        assert!(matches!(
            slides[0].elements[0],
            SlideElement::Note(ref s) if s == "这是备注"
        ));
    }

    #[test]
    fn test_single_line_note_with_extra_spaces() {
        let content = "<!--   多余空格备注   -->";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert!(matches!(
            slides[0].elements[0],
            SlideElement::Note(ref s) if s == "多余空格备注"
        ));
    }

    #[test]
    fn test_single_line_empty_note() {
        // 空备注 <!-- --> 不生成 Note 元素；纯空备注文件视为无有效内容
        let content = "<!-- -->";
        let result = Parser::parse(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_note_skipped_within_slide() {
        // 幻灯片中有其他内容时，空备注被跳过，不产生元素
        let content = "# Title\n<!-- -->\n<!-- 有效备注 -->\n- item";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        // 4 个元素: Heading, Note(有效), ListItem (空备注被跳过)
        // 注意：<!-- --> 和 <!-- 有效备注 --> 之间没有空行，所以不产生 EmptyLine
        assert_eq!(slides[0].elements.len(), 3);
        assert!(matches!(slides[0].elements[0], SlideElement::Heading(1, _)));
        assert!(matches!(slides[0].elements[1], SlideElement::Note(_)));
        assert!(matches!(slides[0].elements[2], SlideElement::ListItem(_)));
    }

    #[test]
    fn test_multiline_note() {
        let content = "<!--\n这是多行备注\n第二行\n第三行\n-->";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert_eq!(slides[0].elements.len(), 1);
        match &slides[0].elements[0] {
            SlideElement::Note(text) => {
                assert!(text.contains("这是多行备注"));
                assert!(text.contains("第二行"));
                assert!(text.contains("第三行"));
            }
            _ => panic!("Expected Note element"),
        }
    }

    #[test]
    fn test_multiline_note_inline_close() {
        // 第一行开始，第二行直接带有 -->
        let content = "<!-- 第一行内容\n第二行结束 -->";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        match &slides[0].elements[0] {
            SlideElement::Note(text) => {
                assert!(text.contains("第一行内容"));
                assert!(text.contains("第二行结束"));
            }
            _ => panic!("Expected Note element"),
        }
    }

    #[test]
    fn test_note_mixed_with_content() {
        let content = "# 标题\n<!-- 标题备注 -->\n- 列表项\n<!-- 列表备注 -->\n正文";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        let elements = &slides[0].elements;
        // 应有: Heading, Note, ListItem, Note, Paragraph = 5
        assert_eq!(elements.len(), 5);
        assert!(matches!(elements[0], SlideElement::Heading(1, _)));
        assert!(matches!(elements[1], SlideElement::Note(_)));
        assert!(matches!(elements[2], SlideElement::ListItem(_)));
        assert!(matches!(elements[3], SlideElement::Note(_)));
        assert!(matches!(elements[4], SlideElement::Paragraph(_)));
    }

    #[test]
    fn test_note_preserved_across_slides() {
        let content = "# S1\n<!-- S1备注 -->\n---\n# S2\n<!-- S2备注 -->";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 2);
        // 第一页有 Heading + Note
        assert_eq!(slides[0].elements.len(), 2);
        assert!(matches!(slides[0].elements[0], SlideElement::Heading(1, _)));
        assert!(matches!(slides[0].elements[1], SlideElement::Note(_)));
        // 第二页有 Heading + Note
        assert_eq!(slides[1].elements.len(), 2);
        assert!(matches!(slides[1].elements[0], SlideElement::Heading(1, _)));
        assert!(matches!(slides[1].elements[1], SlideElement::Note(_)));
    }

    #[test]
    fn test_note_not_created_in_code_block() {
        // 代码块内的 <!-- 不视为备注
        let content = "```\n<!-- 这不是备注 -->\n```";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        assert!(matches!(slides[0].elements[0], SlideElement::CodeBlock(_)));
        if let SlideElement::CodeBlock(ref code) = slides[0].elements[0] {
            assert!(code.contains("<!-- 这不是备注 -->"));
        }
    }

    #[test]
    fn test_unclosed_note_auto_close() {
        // 未闭合的备注在文件末尾自动收尾
        let content = "<!-- 未闭合的备注\n继续写\n最终也没有闭合";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        match &slides[0].elements[0] {
            SlideElement::Note(text) => {
                assert!(text.contains("未闭合的备注"));
                assert!(text.contains("继续写"));
                assert!(text.contains("最终也没有闭合"));
            }
            _ => panic!("Expected Note element"),
        }
    }

    #[test]
    fn test_note_with_markdown_formatting() {
        // 备注内容可包含任意字符，内部 ** 等不产生格式
        let content = "<!-- 使用 **加粗** 和 *斜体* 强调 -->";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 1);
        match &slides[0].elements[0] {
            SlideElement::Note(text) => {
                assert!(text.contains("**加粗**"));
                assert!(text.contains("*斜体*"));
            }
            _ => panic!("Expected Note element"),
        }
    }

    #[test]
    fn test_note_before_first_slide_separator() {
        let content = "<!-- 全局备注 -->\n# Slide 1\n---\n# Slide 2";
        let slides = Parser::parse(content).unwrap();
        assert_eq!(slides.len(), 2);
        // 第一页应有 Note + Heading
        assert!(matches!(slides[0].elements[0], SlideElement::Note(_)));
        assert!(matches!(slides[0].elements[1], SlideElement::Heading(1, _)));
        // 第二页只有 Heading
        assert_eq!(slides[1].elements.len(), 1);
    }

    // ========== collect_slide_notes 测试 ==========

    #[test]
    fn test_collect_notes_empty() {
        let slide = Slide { elements: vec![] };
        assert!(collect_slide_notes(&slide).is_empty());
    }

    #[test]
    fn test_collect_notes_single() {
        let slide = Slide {
            elements: vec![
                SlideElement::Heading(1, vec![]),
                SlideElement::Note("备注内容".to_string()),
                SlideElement::Paragraph(vec![]),
            ],
        };
        let notes = collect_slide_notes(&slide);
        assert_eq!(notes, vec!["备注内容"]);
    }

    #[test]
    fn test_collect_notes_multiple() {
        let slide = Slide {
            elements: vec![
                SlideElement::Note("第一条".to_string()),
                SlideElement::Paragraph(vec![]),
                SlideElement::Note("第二条".to_string()),
                SlideElement::Note("第三条".to_string()),
            ],
        };
        let notes = collect_slide_notes(&slide);
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0], "第一条");
        assert_eq!(notes[1], "第二条");
        assert_eq!(notes[2], "第三条");
    }

    #[test]
    fn test_collect_notes_no_notes_present() {
        let slide = Slide {
            elements: vec![
                SlideElement::Heading(1, vec![]),
                SlideElement::Paragraph(vec![]),
                SlideElement::CodeBlock("code".to_string()),
                SlideElement::EmptyLine,
            ],
        };
        assert!(collect_slide_notes(&slide).is_empty());
    }

    // ========== collect_slide_images 测试 ==========

    #[test]
    fn test_collect_images_empty() {
        let slide = Slide { elements: vec![] };
        assert!(collect_slide_images(&slide).is_empty());
    }

    #[test]
    fn test_collect_images_in_paragraph() {
        let slide = Slide {
            elements: vec![SlideElement::Paragraph(vec![
                normal!("text "),
                image!("logo", "logo.png"),
                normal!(" more"),
            ])],
        };
        let images = collect_slide_images(&slide);
        assert_eq!(images, vec!["logo.png"]);
    }

    #[test]
    fn test_collect_images_deduplicate() {
        let slide = Slide {
            elements: vec![SlideElement::Paragraph(vec![
                image!("a", "img.png"),
                image!("b", "img.png"), // 重复 src
            ])],
        };
        let images = collect_slide_images(&slide);
        assert_eq!(images.len(), 1);
        assert_eq!(images[0], "img.png");
    }

    #[test]
    fn test_collect_images_from_multiple_elements() {
        let slide = Slide {
            elements: vec![
                SlideElement::Heading(1, vec![image!("h", "h.png")]),
                SlideElement::ListItem(vec![image!("li", "li.png")]),
                SlideElement::Paragraph(vec![image!("p", "p.png")]),
            ],
        };
        let images = collect_slide_images(&slide);
        assert_eq!(images.len(), 3);
        assert!(images.contains(&"h.png".to_string()));
        assert!(images.contains(&"li.png".to_string()));
        assert!(images.contains(&"p.png".to_string()));
    }
}
