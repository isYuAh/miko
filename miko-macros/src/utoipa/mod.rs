pub mod attributes;
/// Utoipa OpenAPI 集成模块
///
/// 提供自动生成 OpenAPI 文档的功能，包括：
/// - 从文档注释提取 summary 和 description
/// - 自动推断参数类型和位置
/// - 自动推断响应类型
/// - 支持用户补充配置
pub mod config;
pub mod generator;
pub mod infer;

#[cfg(test)]
mod tests;
