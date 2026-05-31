use crate::error::{PrismError, Result};

/// 行内文本片段
#[derive(Debug, Clone)]
pub enum TextSpan {
    Normal(String),
    Bold(String),
    Italic(String),
    BoldItalic(String),
    InlineCode(String),
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

/// 幻灯片中的单一视觉元素枚举
#[derive(Debug, Clone)]
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
}

/// 单页幻灯片结构体
#[derive(Debug, Clone)]
pub struct Slide {
    pub elements: Vec<SlideElement>,
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

        // 逐行流式扫描
        for line in content.lines() {
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

            // 状态分支 2：遇到了幻灯片切页符 "---"
            if trimmed == "---" {
                // 如果当前页已经收集了内容，将其打包存入幻灯片大集合，并清空当前页
                if !current_elements.is_empty() {
                    slides.push(Slide { elements: current_elements });
                    current_elements = Vec::new();
                }
                continue;
            }

            // 状态分支 3：解析常规的 Markdown 语法行
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

        // 循环结束后，如果最后一页有残留内容，塞进大集合
        if !current_elements.is_empty() {
            slides.push(Slide { elements: current_elements });
        }

        if slides.is_empty() {
            return Err(PrismError::ParseError(
                "Markdown 文件未包含有效内容，解析失败！".to_string()
            ));
        }

        Ok(slides)
    }
}

pub(crate) fn parse_inline(text: &str) -> Vec<TextSpan> {
    parse_inline_with::<Vec<TextSpan>>(text)
}

fn parse_inline_with<C: InlineSpanCollector + Default>(text: &str) -> C {
    let mut spans = C::default();
    let mut buffer = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

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

fn apply_bold(spans: Vec<TextSpan>) -> Vec<TextSpan> {
    spans
        .into_iter()
        .map(|span| match span {
            TextSpan::Normal(text) => TextSpan::Bold(text),
            TextSpan::Italic(text) => TextSpan::BoldItalic(text),
            TextSpan::Bold(text) => TextSpan::Bold(text),
            TextSpan::BoldItalic(text) => TextSpan::BoldItalic(text),
            TextSpan::InlineCode(text) => TextSpan::InlineCode(text),
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
        })
        .collect()
}

fn flush_normal<C: InlineSpanCollector>(spans: &mut C, buffer: &mut String) {
    if !buffer.is_empty() {
        spans.push_span(TextSpan::Normal(std::mem::take(buffer)));
    }
}
