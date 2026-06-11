use std::time::Duration;

use crate::parser::{Slide, SlideElement, TextSpan, parse_inline};
use crate::theme::ThemeStyles;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// ═══════════════════════════════════════════════════════════════
// 排版与渲染辅助类型/函数（从 render() 闭包中提取）
// ═══════════════════════════════════════════════════════════════

/// 行内片段种类 — 替代魔术数字 0-5，在换行处理中保留样式信息
#[derive(Clone)]
enum SpanKind {
    Normal,
    Bold,
    Italic,
    BoldItalic,
    InlineCode,
    Underline,
}

/// 解析图片标签与可用性：返回 (显示文本, 是否应加下划线)
fn resolve_image_text(alt: &str, src: &str) -> (String, bool) {
    if src.is_empty() {
        return (alt.to_string(), false);
    }
    if let Ok(root) = std::env::current_dir() {
        let path = root.join(src);
        if path.is_file() {
            if alt.is_empty() {
                return (src.to_string(), false);
            }
            return (alt.to_string(), false);
        }
    }
    (src.to_string(), true)
}

/// 解析链接显示文本：有 url 则显示 url，否则显示原始文字
fn resolve_link_text(text: &str, url: &str) -> String {
    if !url.is_empty() {
        return url.to_string();
    }
    text.to_string()
}

/// 将 TextSpan 统一提取为 (SpanKind, String)，供换行函数使用
fn span_to_kind_and_text(span: &TextSpan) -> (SpanKind, String) {
    match span {
        TextSpan::Normal(text) => (SpanKind::Normal, text.clone()),
        TextSpan::Bold(text) => (SpanKind::Bold, text.clone()),
        TextSpan::Italic(text) => (SpanKind::Italic, text.clone()),
        TextSpan::BoldItalic(text) => (SpanKind::BoldItalic, text.clone()),
        TextSpan::InlineCode(text) => (SpanKind::InlineCode, text.clone()),
        TextSpan::Link { text, url } => {
            let label = resolve_link_text(text, url);
            (SpanKind::Normal, label)
        }
        TextSpan::Image { alt, src } => {
            let (label, underline) = resolve_image_text(alt, src);
            if underline {
                (SpanKind::Underline, label)
            } else {
                (SpanKind::Normal, label)
            }
        }
    }
}

/// 计算一组已分类文本片段的 Unicode 显示总宽度
fn inline_width(parts: &[(SpanKind, String)]) -> usize {
    parts.iter().map(|(_, text)| text.width()).sum()
}

/// 从 (SpanKind, String) 片段构建 ratatui Span 列表
fn build_rich_spans(parts: &[(SpanKind, String)], base_style: Style) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for (kind, text) in parts {
        if text.is_empty() {
            continue;
        }
        match kind {
            SpanKind::Normal => {
                out.push(Span::styled(text.clone(), base_style));
            }
            SpanKind::Bold => {
                out.push(Span::styled(
                    text.clone(),
                    base_style.add_modifier(Modifier::BOLD),
                ));
            }
            SpanKind::Italic => {
                out.push(Span::styled(
                    text.clone(),
                    base_style.add_modifier(Modifier::ITALIC),
                ));
            }
            SpanKind::BoldItalic => {
                out.push(Span::styled(
                    text.clone(),
                    base_style.add_modifier(Modifier::BOLD | Modifier::ITALIC),
                ));
            }
            SpanKind::InlineCode => {
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
                                out.push(Span::styled(
                                    inner_text,
                                    code_base.add_modifier(Modifier::BOLD),
                                ));
                            }
                        }
                        TextSpan::Italic(inner_text) => {
                            if !inner_text.is_empty() {
                                out.push(Span::styled(
                                    inner_text,
                                    code_base.add_modifier(Modifier::ITALIC),
                                ));
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
                        TextSpan::Link {
                            text: link_text, ..
                        } => {
                            if !link_text.is_empty() {
                                out.push(Span::styled(link_text, code_base));
                            }
                        }
                        TextSpan::Image { alt, src } => {
                            let (label, _underline) = resolve_image_text(&alt, &src);
                            if !label.is_empty() {
                                out.push(Span::styled(label, code_base));
                            }
                        }
                    }
                }
            }
            SpanKind::Underline => {
                out.push(Span::styled(
                    text.clone(),
                    base_style.add_modifier(Modifier::UNDERLINED),
                ));
            }
        }
    }
    out
}

/// 行内文本换行：将 TextSpan 切片按 max_width 换行，返回 (SpanKind, String) 以保留样式
fn wrap_spans(parts: &[TextSpan], max_width: usize) -> Vec<Vec<(SpanKind, String)>> {
    let mut lines: Vec<Vec<(SpanKind, String)>> = Vec::new();
    let mut current: Vec<(SpanKind, String)> = Vec::new();
    let mut used = 0usize;

    for part in parts {
        let (kind, text) = span_to_kind_and_text(part);

        let mut chunk = String::new();
        for ch in text.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + ch_width > max_width && used > 0 {
                if !chunk.is_empty() {
                    current.push((kind.clone(), std::mem::take(&mut chunk)));
                }
                lines.push(std::mem::take(&mut current));
                used = 0;
            }
            chunk.push(ch);
            used += ch_width;
        }

        if !chunk.is_empty() {
            current.push((kind, chunk));
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(Vec::new());
    }

    lines
}

/// 带前缀的行内文本换行：首行用 first_prefix，续行用 cont_prefix
fn wrap_with_prefix(
    spans: &[TextSpan],
    first_prefix: &str,
    cont_prefix: &str,
    max_width: usize,
) -> Vec<(String, Vec<(SpanKind, String)>)> {
    let first_width = first_prefix.width();
    let cont_width = cont_prefix.width();
    let mut lines: Vec<(String, Vec<(SpanKind, String)>)> = Vec::new();
    let mut current: Vec<(SpanKind, String)> = Vec::new();
    let mut used = first_width;
    let mut is_first = true;

    for part in spans {
        let (kind, text) = span_to_kind_and_text(part);

        let mut chunk = String::new();
        for ch in text.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + ch_width > max_width && used > if is_first { first_width } else { cont_width }
            {
                if !chunk.is_empty() {
                    current.push((kind.clone(), std::mem::take(&mut chunk)));
                }
                let prefix = if is_first { first_prefix } else { cont_prefix };
                lines.push((prefix.to_string(), std::mem::take(&mut current)));
                is_first = false;
                used = cont_width;
            }
            chunk.push(ch);
            used += ch_width;
        }

        if !chunk.is_empty() {
            current.push((kind, chunk));
        }
    }

    let prefix = if is_first { first_prefix } else { cont_prefix };
    lines.push((prefix.to_string(), current));
    lines
}

/// 将 Duration 格式化为 HH:MM:SS
fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// 渲染演讲者右侧小抄面板（计时器 + 备注列表）
fn render_presenter_panel(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    notes: &[String],
    elapsed: Duration,
    theme: &ThemeStyles,
) {
    let panel_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_color))
        .title(" 演讲者小抄 ")
        .title_alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(theme.bg_color));

    let inner = panel_block.inner(area);
    f.render_widget(panel_block, area);

    let panel_width = inner.width as usize;
    let mut lines = Vec::new();

    // 计时器行
    let timer_text = format!(" ⏱  {}", format_duration(elapsed));
    lines.push(Line::from(Span::styled(
        timer_text,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        " ──────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::default());

    // 备注内容
    if notes.is_empty() {
        lines.push(Line::from(Span::styled(
            " （无备注）",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for note in notes {
            // 每条备注用 parse_inline + wrap_spans 排版
            let spans = parse_inline(note);
            for line_parts in wrap_spans(&spans, panel_width.saturating_sub(2)) {
                lines.push(Line::from(build_rich_spans(
                    &line_parts,
                    Style::default().fg(theme.paragraph_style.fg.unwrap_or(Color::White)),
                )));
            }
            // 备注之间留空行
            lines.push(Line::default());
        }
    }

    f.render_widget(Paragraph::new(lines), inner);
}

// ═══════════════════════════════════════════════════════════════

/// 渲染上下文：将 `render()` 的 7 个配置参数收拢为单一结构体
pub struct RenderContext<'a> {
    pub slide: &'a Slide,
    pub theme: &'a ThemeStyles,
    pub current_page: usize,
    pub total_pages: usize,
    pub goto_input: Option<&'a str>,
    pub elapsed: Duration,
    pub presenter_notes: Option<&'a [String]>,
}

/// TUI 视觉渲染中心总入口
pub fn render(f: &mut Frame, ctx: &RenderContext<'_>) {
    // 1. 铺满整屏背景色
    let full_area = f.size();
    let bg_block = Block::default().style(Style::default().bg(ctx.theme.bg_color));
    f.render_widget(bg_block, full_area);

    // 2. 运用网格布局切分为主舞台和底部
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(full_area);

    // 3. 根据是否有演讲者模式，决定内容区是否需要左右分栏
    let is_presenter = ctx.presenter_notes.is_some();

    let (slide_area, note_area) = if is_presenter {
        let h_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(chunks[0]);
        (h_chunks[0], Some(h_chunks[1]))
    } else {
        (chunks[0], None)
    };

    // 渲染主舞台大外框
    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ctx.theme.border_color))
        .title(format!(
            " Prism Presentation | 第 {}/{} 页 ",
            ctx.current_page + 1,
            ctx.total_pages,
        ))
        .title_alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(ctx.theme.bg_color));

    let inner_area = main_block.inner(slide_area);
    f.render_widget(main_block, slide_area);

    // 4. 排版控制：逐元素构建 display_lines
    let mut display_lines = Vec::new();
    display_lines.push(Line::default()); // 顶部留白

    let max_line_width = inner_area.width as usize;

    for element in &ctx.slide.elements {
        match element {
            SlideElement::Heading(level, spans) => {
                let heading_style = match level {
                    1 => ctx
                        .theme
                        .heading_style
                        .add_modifier(Modifier::BOLD | Modifier::ITALIC),
                    2 => ctx.theme.heading_style.add_modifier(Modifier::BOLD),
                    3..=6 => ctx.theme.heading_style,
                    _ => ctx.theme.heading_style,
                };
                let indent = match level {
                    1 | 2 => "",
                    3 => "  ",
                    4 => "    ",
                    5 => "      ",
                    6 => "        ",
                    _ => "        ",
                };
                for (prefix, line_parts) in wrap_with_prefix(spans, indent, indent, max_line_width)
                {
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
                    line_spans.push(Span::styled(prefix, ctx.theme.list_style));
                    line_spans.extend(build_rich_spans(&line_parts, ctx.theme.paragraph_style));
                    display_lines.push(Line::from(line_spans));
                }
            }

            SlideElement::Paragraph(spans) => {
                for line_parts in wrap_spans(spans, max_line_width) {
                    display_lines.push(Line::from(build_rich_spans(
                        &line_parts,
                        ctx.theme.paragraph_style,
                    )));
                }
            }

            SlideElement::CodeBlock(code) => {
                display_lines.push(Line::default());

                // 基于原始行宽计算代码框宽度（取最长行）
                let max_len = code.lines().map(|l| l.width()).max().unwrap_or(0);
                let max_box_width = max_line_width.saturating_sub(4).max(10);
                let box_width = (max_len + 4).min(max_box_width).max(10);

                // 代码框天花板 ┌──────────┐
                let top_border = format!("  ┌{}┐", "─".repeat(box_width - 2));
                display_lines.push(Line::from(Span::styled(top_border, ctx.theme.code_style)));

                // 逐行填入，两侧 │ 包裹，右侧空格补齐
                for code_line in code.lines() {
                    let spans = parse_inline(code_line);
                    for line_parts in wrap_spans(&spans, box_width - 4) {
                        let content_width = inline_width(&line_parts);
                        let padding_size = (box_width - 4).saturating_sub(content_width);

                        let mut line_spans = Vec::new();
                        line_spans.push(Span::styled("  │ ", ctx.theme.code_style));
                        line_spans.extend(build_rich_spans(&line_parts, ctx.theme.code_style));
                        if padding_size > 0 {
                            line_spans
                                .push(Span::styled(" ".repeat(padding_size), ctx.theme.code_style));
                        }
                        line_spans.push(Span::styled(" │", ctx.theme.code_style));
                        display_lines.push(Line::from(line_spans));
                    }
                }

                // 代码框地板 └──────────┘
                let bottom_border = format!("  └{}┘", "─".repeat(box_width - 2));
                display_lines.push(Line::from(Span::styled(
                    bottom_border,
                    ctx.theme.code_style,
                )));

                display_lines.push(Line::default());
            }

            SlideElement::EmptyLine => {
                display_lines.push(Line::default());
            }

            SlideElement::Note(_) => {
                // 备注不在主舞台显示，演讲者模式下由右侧面板渲染
            }
        }
    }

    let content_paragraph = Paragraph::new(display_lines);
    f.render_widget(content_paragraph, inner_area);

    // 4b. 演讲者模式：渲染右侧小抄面板
    if let (Some(area), Some(notes)) = (note_area, ctx.presenter_notes) {
        render_presenter_panel(f, area, notes, ctx.elapsed, ctx.theme);
    }

    // 5. 渲染底部状态栏（跳转模式覆盖常规提示）
    let status_string = if let Some(input) = ctx.goto_input {
        if input.is_empty() {
            " 跳转到: _  (输入页码后按 Enter，Esc 取消)".to_string()
        } else {
            format!(" 跳转到: {}_  (Enter 确认，Esc 取消)", input)
        }
    } else {
        let progress = (ctx.current_page + 1) as f32 / ctx.total_pages as f32;
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
        let tips_part = "   |  操作提示: [←] 上一页  [→] 下一页  [G] 跳转页  [Q/ESC] 退出播放";
        let full_status = format!("{}{}", progress_part, tips_part);
        if full_status.width() > full_area.width as usize {
            progress_part
        } else {
            full_status
        }
    };

    let status_line = Line::from(Span::styled(
        status_string,
        Style::default()
            .fg(ctx.theme.border_color)
            .bg(ctx.theme.bg_color),
    ));

    f.render_widget(
        Paragraph::new(status_line).alignment(ratatui::layout::Alignment::Center),
        chunks[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== format_duration 测试 ==========

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00");
    }

    #[test]
    fn test_format_duration_seconds_only() {
        assert_eq!(format_duration(Duration::from_secs(7)), "00:07");
        assert_eq!(format_duration(Duration::from_secs(42)), "00:42");
    }

    #[test]
    fn test_format_duration_minutes_and_seconds() {
        assert_eq!(format_duration(Duration::from_secs(65)), "01:05");
        assert_eq!(format_duration(Duration::from_secs(125)), "02:05");
        assert_eq!(format_duration(Duration::from_secs(599)), "09:59");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59:59");
    }

    #[test]
    fn test_format_duration_exactly_one_hour() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1:00:00");
    }

    #[test]
    fn test_format_duration_hours_minutes_seconds() {
        assert_eq!(format_duration(Duration::from_secs(3661)), "1:01:01");
        assert_eq!(format_duration(Duration::from_secs(3725)), "1:02:05");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2:00:00");
        assert_eq!(format_duration(Duration::from_secs(7384)), "2:03:04");
    }

    #[test]
    fn test_format_duration_large_hours() {
        assert_eq!(format_duration(Duration::from_secs(36000)), "10:00:00");
        assert_eq!(format_duration(Duration::from_secs(36610)), "10:10:10");
    }
}
