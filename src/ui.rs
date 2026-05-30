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

    let make_span = |marker: u8, text: String| -> TextSpan {
        match marker {
            0 => TextSpan::Normal(text),
            1 => TextSpan::Bold(text),
            2 => TextSpan::Italic(text),
            3 => TextSpan::BoldItalic(text),
            _ => TextSpan::InlineCode(text),
        }
    };

    let wrap_spans = |parts: &[TextSpan], max_width: usize| -> Vec<Vec<TextSpan>> {
        let mut lines = Vec::new();
        let mut current = Vec::new();
        let mut used = 0usize;

        for part in parts {
            let (text, marker) = match part {
                TextSpan::Normal(text) => (text, 0),
                TextSpan::Bold(text) => (text, 1),
                TextSpan::Italic(text) => (text, 2),
                TextSpan::BoldItalic(text) => (text, 3),
                TextSpan::InlineCode(text) => (text, 4),
            };

            let mut chunk = String::new();
            for ch in text.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if used + ch_width > max_width && used > 0 {
                    if !chunk.is_empty() {
                        current.push(make_span(marker, chunk));
                        chunk = String::new();
                    }
                    lines.push(current);
                    current = Vec::new();
                    used = 0;
                }
                chunk.push(ch);
                used += ch_width;
            }

            if !chunk.is_empty() {
                current.push(make_span(marker, chunk));
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }

        if lines.is_empty() {
            lines.push(Vec::new());
        }

        lines
    };

    let wrap_with_prefix = |spans: &[TextSpan], first_prefix: &str, cont_prefix: &str, max_width: usize| -> Vec<(String, Vec<TextSpan>)> {
        let first_width = first_prefix.width();
        let cont_width = cont_prefix.width();
        let mut lines = Vec::new();
        let mut current = Vec::new();
        let mut used = first_width;
        let mut is_first = true;

        for part in spans {
            let (text, marker) = match part {
                TextSpan::Normal(text) => (text, 0),
                TextSpan::Bold(text) => (text, 1),
                TextSpan::Italic(text) => (text, 2),
                TextSpan::BoldItalic(text) => (text, 3),
                TextSpan::InlineCode(text) => (text, 4),
            };

            let mut chunk = String::new();
            for ch in text.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if used + ch_width > max_width && used > if is_first { first_width } else { cont_width } {
                    if !chunk.is_empty() {
                        current.push(make_span(marker, chunk));
                        chunk = String::new();
                    }
                    let prefix = if is_first { first_prefix } else { cont_prefix };
                    lines.push((prefix.to_string(), current));
                    current = Vec::new();
                    is_first = false;
                    used = cont_width;
                }
                chunk.push(ch);
                used += ch_width;
            }

            if !chunk.is_empty() {
                current.push(make_span(marker, chunk));
            }
        }

        let prefix = if is_first { first_prefix } else { cont_prefix };
        lines.push((prefix.to_string(), current));
        lines
    };

    let max_line_width = inner_area.width as usize;

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
                for (prefix, line_parts) in wrap_with_prefix(spans, indent, indent, max_line_width) {
                    let mut line_spans = Vec::new();
                    if !prefix.is_empty() {
                        line_spans.push(Span::styled(prefix, heading_style));
                    }
                    line_spans.extend(build_rich_spans(&line_parts, heading_style));
                    display_lines.push(Line::from(line_spans));
                }
                display_lines.push(Line::default());
            }

            SlideElement::ListItem(spans) => {
                let wrapped = wrap_with_prefix(spans, "  • ", "    ", max_line_width);
                for (prefix, line_parts) in wrapped {
                    let mut line_spans = Vec::new();
                    line_spans.push(Span::styled(prefix, theme.list_style));
                    line_spans.extend(build_rich_spans(&line_parts, theme.paragraph_style));
                    display_lines.push(Line::from(line_spans));
                }
            }

            SlideElement::Paragraph(spans) => {
                for line_parts in wrap_spans(spans, max_line_width) {
                    display_lines.push(Line::from(build_rich_spans(&line_parts, theme.paragraph_style)));
                }
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
                // 确定代码框的动态物理总宽度（左右各预留安全空间，限制在可视宽度内）
                let max_box_width = max_line_width.saturating_sub(4).max(10);
                let box_width = (max_len + 4).min(max_box_width).max(10);

                // 画出代码块的字形天花板 ┌─────────────────┐
                let top_border = format!("  ┌{}┐", "─".repeat(box_width - 2));
                display_lines.push(Line::from(Span::styled(top_border, theme.code_style)));

                // 逐行把代码填入方框，两侧用 │ 字符严密包裹，右侧自动用空格精准补齐
                for code_line in code.lines() {
                    let spans = parse_inline(code_line);
                    for line_parts in wrap_spans(&spans, box_width - 4) {
                        let content_width = inline_width(&line_parts);
                        let padding_size = (box_width - 4).saturating_sub(content_width);

                        let mut line_spans = Vec::new();
                        line_spans.push(Span::styled("  │ ", theme.code_style));
                        line_spans.extend(build_rich_spans(&line_parts, theme.code_style));
                        if padding_size > 0 {
                            line_spans.push(Span::styled(" ".repeat(padding_size), theme.code_style));
                        }
                        line_spans.push(Span::styled(" │", theme.code_style));
                        display_lines.push(Line::from(line_spans));
                    }
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
