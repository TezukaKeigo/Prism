use clap::Parser;
use std::path::PathBuf;
use crate::error::{PrismError, Result};

/// 命令行参数解析结构体
#[derive(Parser, Debug)]
#[command(name = "prism")]
#[command(version = "0.1.0")]
#[command(about = "Prism —— 基于 TUI 的轻量级 Markdown 交互式演讲工具", long_about = None)]
pub struct Config {
    /// Markdown 文件路径（必须提供）
    #[arg(value_name = "FILE_PATH")]
    pub file_path: PathBuf,

    /// 可选参数：演示界面主题（如 default, matrix, dracula），默认使用 default
    #[arg(short, long, default_value = "default")]
    pub theme: String,
}

impl Config {
    /// 解析命令行参数，并进行基础的文件存在性验证
    pub fn parse_args() -> Result<Self> {
        // 自动从终端抓取并解析参数
        let config = Self::parse();

        // 如果用户传的文件路径根本不存在，直接拦截并报错
        if !config.file_path.exists() {
            return Err(PrismError::ConfigError(format!(
                "指定的文件不存在或无法访问: {:?}",
                config.file_path
            )));
        }

        // 返回配置对象
        Ok(config)
    }
}