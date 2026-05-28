use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Style, Modifier, Color},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthStr;
use crate::parser::{Slide, SlideElement};
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

    for element in &slide.elements {
        match element {
            // 利用 match 拉开多级标题的视觉层级
            SlideElement::Heading(level, text) => {
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
                display_lines.push(Line::from(vec![
                    Span::styled(indent, heading_style),
                    Span::styled(text.clone(), heading_style),
                ]));
                display_lines.push(Line::default());
            }
            
            SlideElement::ListItem(text) => {
                display_lines.push(Line::from(vec![
                    Span::styled("  • ", theme.list_style),
                    Span::styled(text.clone(), theme.paragraph_style),
                ]));
            }
            
            SlideElement::Paragraph(text) => {
                display_lines.push(Line::from(Span::styled(text.clone(), theme.paragraph_style)));
            }
            
            SlideElement::CodeBlock(code) => {
                display_lines.push(Line::default());

                // 平滑扫描当前代码块，动态计算出最长的一行有多少个字符
                let mut max_len = 0;
                for line in code.lines() {
                    if line.len() > max_len {
                        max_len = line.len();
                    }
                }
                // 确定代码框的动态物理总宽度（左右各预留安全空间，限制在 30~70 字符宽）
                let box_width = (max_len + 4).max(30).min(70);

                // 画出代码块的字形天花板 ┌─────────────────┐
                let top_border = format!("  ┌{}┐", "─".repeat(box_width - 2));
                display_lines.push(Line::from(Span::styled(top_border, theme.code_style)));

                // 逐行把代码填入方框，两侧用 │ 字符严密包裹，右侧自动用空格精准补齐
                for code_line in code.lines() {
                    let mut line_content = code_line.to_string();
                    // 如果单行代码太长，强行截断；如果不够长，拼命补空格对齐右边界
                    if line_content.len() > box_width - 4 {
                        line_content.truncate(box_width - 7);
                        line_content.push_str("...");
                    }
                    let padding_size = (box_width - 4) - line_content.len();
                    let padded_line = format!("  │ {}{} │", line_content, " ".repeat(padding_size));
                    
                    display_lines.push(Line::from(Span::styled(padded_line, theme.code_style)));
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