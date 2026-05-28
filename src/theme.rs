use ratatui::style::{Color, Style, Modifier};

/// 统一管理幻灯片各类视觉元素的样式包
pub struct ThemeStyles {
    /// 整个终端背景色
    pub bg_color: Color,
    /// 标题样式（# 级别的文字）
    pub heading_style: Style,
    /// 普通正文段落样式
    pub paragraph_style: Style,
    /// 列表项样式
    pub list_style: Style,
    /// 代码块样式（文字颜色与代码框内背景色）
    pub code_style: Style,
    /// 幻灯片外围大边框的颜色
    pub border_color: Color,
}

impl ThemeStyles {
    /// 根据用户输入的主题名称，匹配并生成一整套色彩样式
    pub fn new(theme_name: &str) -> Self {
        match theme_name.to_lowercase().as_str() {
            // 皮肤 1：黑客帝国 (Matrix) —— 满屏纯黑与荧光绿
            "matrix" => Self {
                bg_color: Color::Black,
                heading_style: Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                paragraph_style: Style::default().fg(Color::Green),
                list_style: Style::default().fg(Color::Yellow),
                code_style: Style::default().fg(Color::Yellow).bg(Color::Black),
                border_color: Color::Green,
            },

            // 皮肤 2：德拉库拉 (Dracula) —— 国际著名的顶级吸血鬼暗黑主题
            "dracula" => Self {
                bg_color: Color::Rgb(40, 42, 54), // 经典暗紫底色
                heading_style: Style::default().fg(Color::Rgb(189, 147, 249)).add_modifier(Modifier::BOLD), 
                paragraph_style: Style::default().fg(Color::Rgb(248, 248, 242)), 
                list_style: Style::default().fg(Color::Yellow), 
                code_style: Style::default().fg(Color::Yellow).bg(Color::Rgb(40, 42, 54)), 
                border_color: Color::Rgb(98, 114, 164), 
            },

            // 皮肤 3：默认 (Default) —— 系统级全兼容极客蓝（换用工业安全色）
            _ => Self {
                bg_color: Color::Black, // 纯黑底，任何终端都不会错位
                heading_style: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD), 
                paragraph_style: Style::default().fg(Color::White), 
                list_style: Style::default().fg(Color::Yellow), 
                code_style: Style::default().fg(Color::Yellow).bg(Color::Black),
                border_color: Color::Blue, 
            },
        }
    }
}