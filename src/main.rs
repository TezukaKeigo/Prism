mod config;
mod error;
mod parser;
mod theme;
mod ui;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::error::{PrismError, Result};
use crate::parser::{Parser, collect_slide_images, collect_slide_notes};
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
    let mut terminal =
        Terminal::new(backend).map_err(|e| PrismError::TerminalError(e.to_string()))?;
    terminal
        .clear()
        .map_err(|e| PrismError::TerminalError(e.to_string()))?;

    // 4. 播放主循环
    let mut current_page = 0;
    let mut should_quit = false;

    // G 键跳转模式状态
    let mut in_goto_mode = false;
    let mut goto_buffer = String::new();

    // 演讲计时器起点
    let start_time = Instant::now();
    let presenter = config.presenter;

    while !should_quit {
        // 计算实时经过时长
        let elapsed = start_time.elapsed();

        // 构建跳转输入提示（用于 UI 渲染）
        let goto_input: Option<&str> = if in_goto_mode {
            Some(goto_buffer.as_str())
        } else {
            None
        };

        // 演讲者模式：提取当前页备注
        let slide_notes: Option<Vec<String>> = if presenter {
            let notes = collect_slide_notes(&slides[current_page]);
            Some(notes)
        } else {
            None
        };

        // 将当前页数据、主题、页码、跳转状态、计时、备注传入 UI 模块
        let render_ctx = ui::RenderContext {
            slide: &slides[current_page],
            theme: &theme_styles,
            current_page,
            total_pages,
            goto_input,
            elapsed,
            presenter_notes: slide_notes.as_deref(),
        };

        terminal
            .draw(|f| {
                ui::render(f, &render_ctx);
            })
            .map_err(|e| PrismError::TerminalError(e.to_string()))?;

        if event::poll(Duration::from_millis(100))
            .map_err(|e| PrismError::TerminalError(e.to_string()))?
            && let Event::Key(key) =
                event::read().map_err(|e| PrismError::TerminalError(e.to_string()))?
            && key.kind == event::KeyEventKind::Press
        {
            // ── 跳转模式分支 ──
            if in_goto_mode {
                match key.code {
                    KeyCode::Esc => {
                        // 取消跳转
                        in_goto_mode = false;
                        goto_buffer.clear();
                    }
                    KeyCode::Enter => {
                        // 确认跳转：解析数字并跳页
                        if let Ok(target) = goto_buffer.parse::<usize>() {
                            // 用户输入为 1-based，转换为 0-based 并钳制
                            let page = if target == 0 {
                                0
                            } else {
                                (target - 1).min(total_pages - 1)
                            };
                            current_page = page;
                        }
                        in_goto_mode = false;
                        goto_buffer.clear();
                    }
                    KeyCode::Backspace => {
                        goto_buffer.pop();
                    }
                    KeyCode::Char(ch) if ch.is_ascii_digit() => {
                        goto_buffer.push(ch);
                    }
                    _ => {} // 忽略其他按键
                }
            } else {
                // ── 正常模式分支 ──
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        should_quit = true;
                    }
                    KeyCode::Right | KeyCode::Char(' ') => {
                        if current_page < total_pages - 1 {
                            current_page += 1;
                        }
                    }
                    KeyCode::Left => {
                        current_page = current_page.saturating_sub(1);
                    }
                    // 按 G 键进入跳转模式
                    KeyCode::Char('g') | KeyCode::Char('G') => {
                        in_goto_mode = true;
                        goto_buffer.clear();
                    }
                    // 按 O 打开当前页图片（若存在）
                    KeyCode::Char('o') | KeyCode::Char('O') => {
                        let images = collect_slide_images(&slides[current_page]);
                        if let Some(path) = images.first() {
                            open_image(path)?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn open_image(path: &str) -> Result<()> {
    let full_path = std::env::current_dir()
        .map_err(PrismError::IoError)?
        .join(path);
    if !full_path.exists() {
        return Err(PrismError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("图片不存在: {}", full_path.display()),
        )));
    }

    Command::new("cmd")
        .args(["/C", "start", "", &full_path.to_string_lossy()])
        .spawn()
        .map_err(PrismError::IoError)?;

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
