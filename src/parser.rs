use crate::error::{PrismError, Result};

/// 幻灯片中的单一视觉元素枚举
#[derive(Debug, Clone)]
pub enum SlideElement {
    /// 标题(级别 1-6, 文本内容)
    Heading(u8, String),
    /// 列表项(文本内容)
    ListItem(String),
    /// 代码块(代码文本内容)
    CodeBlock(String),
    /// 普通段落文本
    Paragraph(String),
    /// 空行（用于在 TUI 渲染时保留物理排版间距）
    EmptyLine,
}

/// 单页幻灯片结构体
#[derive(Debug, Clone)]
pub struct Slide {
    pub elements: Vec<SlideElement>,
}

/// Markdown 解析器
pub struct Parser;

impl Parser {
    /// 将整个 Markdown 文本流式解析为一组幻灯片页
    pub fn parse(content: &str) -> Result<Vec<Slide>> {
        let mut slides = Vec::new();
        let mut current_elements = Vec::new();
        
        // 状态机标志：指示当前是否正处于一个多行代码块里面
        let mut in_code_block = false;
        let mut current_code_lines = Vec::new();

        // 逐行流式扫描
        for line in content.lines() {
            let trimmed = line.trim();

            // 状态分支 1：如果当前正处于代码块内部
            if in_code_block {
                if trimmed.starts_with("```") {
                    // 遇到代码块闭合标记，关闭状态机，打包代码块内容
                    in_code_block = false;
                    let code_content = current_code_lines.join("\n");
                    current_elements.push(SlideElement::CodeBlock(code_content));
                    current_code_lines.clear();
                } else {
                    // 还在代码块内部，保留原始行
                    current_code_lines.push(line.to_string());
                }
                continue;
            }

            // 状态分支 2：遇到了幻灯片切页符 "---"
            if trimmed == "---" {
                // 如果当前页已经收集了内容，将其打包存入幻灯片大集合，并清空当前页
                if !current_elements.is_empty() {
                    slides.push(Slide { elements: current_elements });
                    current_elements = Vec::new();
                }
                continue;
            }

            // 状态分支 3：解析常规的 Markdown 语法行
            if trimmed.starts_with("```") {
                // 触发代码块开启状态
                in_code_block = true;
            } else if trimmed.starts_with('#') {
                // 计算 # 的数量从而确定标题级别
                let mut level = 0;
                for ch in line.chars() {
                    if ch == '#' { level += 1; } else { break; }
                }
                
                let content_part = line.trim_start_matches('#');
                // 严格标准：# 后面必须有一个空格才算合法标题，且级别不超过 6
                if content_part.starts_with(' ') && level <= 6 {
                    current_elements.push(SlideElement::Heading(level as u8, content_part.trim().to_string()));
                } else {
                    current_elements.push(SlideElement::Paragraph(line.to_string()));
                }
            } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                // 识别无序列表项，裁剪掉前缀符号
                let item_content = trimmed[2..].trim().to_string();
                current_elements.push(SlideElement::ListItem(item_content));
            } else if trimmed.is_empty() {
                // 收集空行，防止 PPT 所有内容死死贴在一起，留出呼吸感
                current_elements.push(SlideElement::EmptyLine);
            } else {
                // 兜底机制：其余全部认作普通文本段落
                current_elements.push(SlideElement::Paragraph(line.to_string()));
            }
        }

        // 循环结束后，如果还在代码块内，强制收尾并打包
        if in_code_block {
            let code_content = current_code_lines.join("\n");
            current_elements.push(SlideElement::CodeBlock(code_content));
            current_code_lines.clear();
        }

        // 循环结束后，如果最后一页有残留内容，塞进大集合
        if !current_elements.is_empty() {
            slides.push(Slide { elements: current_elements });
        }

  
        if slides.is_empty() {
            return Err(PrismError::ParseError(
                "Markdown 文件未包含有效内容，解析失败！".to_string()
            ));
        }

        Ok(slides)
    }
}