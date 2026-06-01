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

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    // 1. 解析命令行参数（如果文件不存在，这里会直接拦截并报错）
    let config = Config::parse_args()?;

    // 2. 读取并解析 Markdown 文件内容
    let content = fs::read_to_string(&config.file_path)?;
    let slides = Parser::parse(&content)?;

    let total_pages = slides.len();
    let theme_styles = ThemeStyles::new(&config.theme);

    // 3. 初始化终端，进入全屏原始模式
    enable_raw_mode().map_err(|e| PrismError::TerminalError(e.to_string()))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| PrismError::TerminalError(e.to_string()))?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| PrismError::TerminalError(e.to_string()))?;
    terminal.clear().map_err(|e| PrismError::TerminalError(e.to_string()))?;

    // 4. 播放主循环
    let mut current_page = 0;
    let mut should_quit = false;

    while !should_quit {
        // 将当前页数据、主题、页码传入 UI 模块
        terminal
            .draw(|f| {
                ui::render(f, &slides[current_page], &theme_styles, current_page, total_pages);
            })
            .map_err(|e| PrismError::TerminalError(e.to_string()))?;

        if event::poll(Duration::from_millis(100)).map_err(|e| PrismError::TerminalError(e.to_string()))? {
            if let Event::Key(key) = event::read().map_err(|e| PrismError::TerminalError(e.to_string()))? {
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

    Ok(())
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
}
