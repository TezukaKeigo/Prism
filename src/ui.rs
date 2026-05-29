use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Style, Modifier, Color},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use crate::parser::{Slide, SlideElement, TextSpan, parse_inline};
use crate::theme::ThemeStyles;

/// TUI 视觉渲染中心总入口
pub fn render(
    f: &mut Frame, 
    slide: &Slide, 
    theme: &ThemeStyles, 
    current_page: usize, 
    total_pages: usize
) {
    // 1. 铺满整屏背景色
    let full_area = f.size();
    let bg_block = Block::default().style(Style::default().bg(theme.bg_color));
    f.render_widget(bg_block, full_area);

    // 2. 运用网格布局切分为主舞台和底部
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(full_area);

    // 3. 渲染主舞台大外框
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_color))
        .title(format!(" Prism Presentation | 第 {}/{} 页 ", current_page + 1, total_pages))
        .title_alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(theme.bg_color));

    let inner_area = main_block.inner(chunks[0]);
    f.render_widget(main_block, chunks[0]);

    // 4. 【升级版排版控制中心】
    let mut display_lines = Vec::new();
    display_lines.push(Line::default()); // 顶部留白

    let build_rich_spans = |parts: &[TextSpan], base_style: Style| -> Vec<Span<'static>> {
        let mut out = Vec::new();
        for part in parts {
            match part {
                TextSpan::Normal(text) => {
                    if !text.is_empty() {
                        out.push(Span::styled(text.clone(), base_style));
                    }
                }
                TextSpan::Bold(text) => {
                    if !text.is_empty() {
                        out.push(Span::styled(text.clone(), base_style.add_modifier(Modifier::BOLD)));
                    }
                }
                TextSpan::Italic(text) => {
                    if !text.is_empty() {
                        out.push(Span::styled(text.clone(), base_style.add_modifier(Modifier::ITALIC)));
                    }
                }
                TextSpan::BoldItalic(text) => {
                    if !text.is_empty() {
                        out.push(Span::styled(
                            text.clone(),
                            base_style.add_modifier(Modifier::BOLD | Modifier::ITALIC),
                        ));
                    }
                }
                TextSpan::InlineCode(text) => {
                    if !text.is_empty() {
                        let code_base = Style::default().fg(Color::Yellow).bg(Color::DarkGray);
                        let inner_parts = parse_inline(text);
                        for inner in inner_parts {
                            match inner {
                                TextSpan::Normal(inner_text) => {
                                    if !inner_text.is_empty() {
                                        out.push(Span::styled(inner_text, code_base));
                                    }
                                }
                                TextSpan::Bold(inner_text) => {
                                    if !inner_text.is_empty() {
                                        out.push(Span::styled(inner_text, code_base.add_modifier(Modifier::BOLD)));
                                    }
                                }
                                TextSpan::Italic(inner_text) => {
                                    if !inner_text.is_empty() {
                                        out.push(Span::styled(inner_text, code_base.add_modifier(Modifier::ITALIC)));
                                    }
                                }
                                TextSpan::BoldItalic(inner_text) => {
                                    if !inner_text.is_empty() {
                                        out.push(Span::styled(
                                            inner_text,
                                            code_base.add_modifier(Modifier::BOLD | Modifier::ITALIC),
                                        ));
                                    }
                                }
                                TextSpan::InlineCode(inner_text) => {
                                    if !inner_text.is_empty() {
                                        out.push(Span::styled(inner_text, code_base));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    };

    let inline_width = |parts: &[TextSpan]| -> usize {
        parts
            .iter()
            .map(|span| match span {
                TextSpan::Normal(text)
                | TextSpan::Bold(text)
                | TextSpan::Italic(text)
                | TextSpan::BoldItalic(text)
                | TextSpan::InlineCode(text) => text.width(),
            })
            .sum()
    };

    let truncate_inline = |parts: &[TextSpan], max_width: usize| -> (Vec<TextSpan>, bool) {
        let mut out = Vec::new();
        let mut used = 0usize;
        let mut truncated = false;

        for part in parts {
            if used >= max_width {
                truncated = true;
                break;
            }

            let (text, marker) = match part {
                TextSpan::Normal(text) => (text, 0),
                TextSpan::Bold(text) => (text, 1),
                TextSpan::Italic(text) => (text, 2),
                TextSpan::BoldItalic(text) => (text, 3),
                TextSpan::InlineCode(text) => (text, 4),
            };

            let mut kept = String::new();
            for ch in text.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if used + ch_width > max_width {
                    truncated = true;
                    break;
                }
                kept.push(ch);
                used += ch_width;
            }

            if !kept.is_empty() {
                let span = match marker {
                    0 => TextSpan::Normal(kept),
                    1 => TextSpan::Bold(kept),
                    2 => TextSpan::Italic(kept),
                    3 => TextSpan::BoldItalic(kept),
                    _ => TextSpan::InlineCode(kept),
                };
                out.push(span);
            }
        }

        (out, truncated)
    };

    for element in &slide.elements {
        match element {
            // 利用 match 拉开多级标题的视觉层级
            SlideElement::Heading(level, spans) => {
                let heading_style = match level {
                    1 => theme.heading_style.add_modifier(Modifier::BOLD | Modifier::ITALIC),
                    2 => theme.heading_style.add_modifier(Modifier::BOLD),
                    3 => theme.heading_style,
                    4 => theme.heading_style,
                    5 => theme.heading_style,
                    6 => theme.heading_style,
                    _ => theme.heading_style,
                };
                let indent = match level {
                    1 => "",
                    2 => "",
                    3 => "  ",
                    4 => "    ",
                    5 => "      ",
                    6 => "        ",
                    _ => "        ",
                };
                let mut line_spans = Vec::new();
                if !indent.is_empty() {
                    line_spans.push(Span::styled(indent, heading_style));
                }
                line_spans.extend(build_rich_spans(spans, heading_style));
                display_lines.push(Line::from(line_spans));
                display_lines.push(Line::default());
            }

            SlideElement::ListItem(spans) => {
                let mut line_spans = vec![Span::styled("  • ", theme.list_style)];
                line_spans.extend(build_rich_spans(spans, theme.paragraph_style));
                display_lines.push(Line::from(line_spans));
            }

            SlideElement::Paragraph(spans) => {
                display_lines.push(Line::from(build_rich_spans(spans, theme.paragraph_style)));
            }

            SlideElement::CodeBlock(code) => {
                display_lines.push(Line::default());

                // 平滑扫描当前代码块，动态计算出最长的一行有多少个字符
                let mut max_len = 0;
                for line in code.lines() {
                    let spans = parse_inline(line);
                    let width = inline_width(&spans);
                    if width > max_len {
                        max_len = width;
                    }
                }
                // 确定代码框的动态物理总宽度（左右各预留安全空间，限制在 30~70 字符宽）
                let box_width = (max_len + 4).max(30).min(70);

                // 画出代码块的字形天花板 ┌─────────────────┐
                let top_border = format!("  ┌{}┐", "─".repeat(box_width - 2));
                display_lines.push(Line::from(Span::styled(top_border, theme.code_style)));

                // 逐行把代码填入方框，两侧用 │ 字符严密包裹，右侧自动用空格精准补齐
                for code_line in code.lines() {
                    let spans = parse_inline(code_line);
                    let (mut trimmed, truncated) = truncate_inline(&spans, box_width - 4);
                    if truncated {
                        trimmed.push(TextSpan::Normal("...".to_string()));
                    }

                    let content_width = inline_width(&trimmed);
                    let padding_size = (box_width - 4).saturating_sub(content_width);

                    let mut line_spans = Vec::new();
                    line_spans.push(Span::styled("  │ ", theme.code_style));
                    line_spans.extend(build_rich_spans(&trimmed, theme.code_style));
                    if padding_size > 0 {
                        line_spans.push(Span::styled(" ".repeat(padding_size), theme.code_style));
                    }
                    line_spans.push(Span::styled(" │", theme.code_style));
                    display_lines.push(Line::from(line_spans));
                }

                // 画出代码块的字形地板 └─────────────────┘
                let bottom_border = format!("  └{}┘", "─".repeat(box_width - 2));
                display_lines.push(Line::from(Span::styled(bottom_border, theme.code_style)));

                display_lines.push(Line::default());
            }

            SlideElement::EmptyLine => {
                display_lines.push(Line::default());
            }
        }
    }

    let content_paragraph = Paragraph::new(display_lines);
    f.render_widget(content_paragraph, inner_area);

    // 5. 渲染底部状态栏进度条
    let progress = (current_page + 1) as f32 / total_pages as f32;
    let bar_width = 25;
    let filled_width = (progress * bar_width as f32).round() as usize;

    let filled_bar = "█".repeat(filled_width);
    let empty_bar = "░".repeat(bar_width - filled_width);

    let progress_part = format!(
        " 进度: [{}{}] {:>3}%",
        filled_bar,
        empty_bar,
        (progress * 100.0) as usize
    );
    let tips_part = "   |  操作提示: [←] 上一页  [→] 下一页  [Q/ESC] 退出播放";
    let full_status = format!("{}{}", progress_part, tips_part);
    let status_string = if full_status.width() > full_area.width as usize {
        progress_part
    } else {
        full_status
    };

    let status_line = Line::from(Span::styled(
        status_string,
        Style::default().fg(theme.border_color).bg(theme.bg_color)
    ));

    f.render_widget(
        Paragraph::new(status_line).alignment(ratatui::layout::Alignment::Center),
        chunks[1],
    );
}
