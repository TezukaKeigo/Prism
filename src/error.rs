use std::fmt;

/// Prism 项目的专属错误枚举
#[derive(Debug)]
pub enum PrismError {
    /// 找不到指定的 Markdown 文件，或者读取失败
    IoError(std::io::Error),
    /// 命令行参数解析错误
    ConfigError(String),
    /// Markdown 语法错误（eg：文件内容为空）
    ParseError(String),
    /// 终端渲染交互期间发生的底层错误
    TerminalError(String),
}

// 为错误实现 Display 特性，打印错误信息时会更友好
impl fmt::Display for PrismError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrismError::IoError(e) => write!(f, "文件读取失败: {e}"),
            PrismError::ConfigError(msg) => write!(f, "配置参数有误: {msg}"),
            PrismError::ParseError(msg) => write!(f, "Markdown 解析异常: {msg}"),
            PrismError::TerminalError(msg) => write!(f, "终端渲染失败: {msg}"),
        }
    }
}

// 实现标准 Error 特性，让错误融入 Rust 标准的错误处理生态中
impl std::error::Error for PrismError {}

// 让 std::io::Error 能够自动隐式转换为 PrismError
impl From<std::io::Error> for PrismError {
    fn from(err: std::io::Error) -> Self {
        PrismError::IoError(err)
    }
}

pub type Result<T> = std::result::Result<T, PrismError>;