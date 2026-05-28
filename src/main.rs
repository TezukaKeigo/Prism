mod config;
mod error;
mod parser;
mod theme;
mod ui;

use std::fs;
use std::time::Duration;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::config::Config;
use crate::error::{PrismError, Result};
use crate::parser::Parser;
use crate::theme::ThemeStyles;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // 1. 解析命令行参数（如果文件不存在，这里会直接拦截并报错）
    let config = match Config::parse_args() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    // 2. 读取并解析 Markdown 文件内容
    let content = fs::read_to_string(&config.file_path)?;
    let slides = match Parser::parse(&content) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };
    
    let total_pages = slides.len();
    let theme_styles = ThemeStyles::new(&config.theme);

    // 3. 初始化终端，进入全屏原始模式
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // 4. 播放主循环
    let mut current_page = 0;
    let mut should_quit = false;

    while !should_quit {
        // 将当前页数据、主题、页码传入 UI 模块
        terminal.draw(|f| {
            ui::render(f, &slides[current_page], &theme_styles, current_page, total_pages);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // 仅响应按下事件，过滤释放事件，防止部分终端双击触发
                if key.kind == event::KeyEventKind::Press {
                    match key.code {
                        // 按 Q 或 ESC 键退出
                        KeyCode::Char('q') | KeyCode::Esc => {
                            should_quit = true;
                        }
                        // 按右方向键或空格，向后翻页
                        KeyCode::Right | KeyCode::Char(' ') => {
                            if current_page < total_pages - 1 {
                                current_page += 1;
                            }
                        }
                        // 按左方向键，向前翻页
                        KeyCode::Left => {
                            if current_page > 0 {
                                current_page -= 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // 5. 程序退出时，必须无条件还原终端状态，否则会导致用户终端卡死或乱码
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
