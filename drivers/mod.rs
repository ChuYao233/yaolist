pub mod local;
pub mod onedrive;
pub mod ftp;
pub mod quark;
pub mod s3;
pub mod cloud189;
pub mod lanzou;
pub mod alist;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use once_cell::sync::Lazy;
use s3::S3DriverFactory;
use cloud189::Cloud189DriverFactory;
use lanzou::LanzouDriverFactory;
use alist::AListDriverFactory;

#[async_trait]
pub trait Driver: Send + Sync {
    async fn list(&self, path: &str) -> anyhow::Result<Vec<FileInfo>>;
    async fn download(&self, path: &str) -> anyhow::Result<tokio::fs::File>;
    async fn get_download_url(&self, path: &str) -> anyhow::Result<Option<String>>;
    async fn upload_file(&self, parent_path: &str, file_name: &str, content: &[u8]) -> anyhow::Result<()>;
    async fn delete(&self, path: &str) -> anyhow::Result<()>;
    async fn rename(&self, path: &str, new_name: &str) -> anyhow::Result<()>;
    async fn create_folder(&self, parent_path: &str, folder_name: &str) -> anyhow::Result<()>;
    async fn get_file_info(&self, path: &str) -> anyhow::Result<FileInfo>;
    async fn move_file(&self, file_path: &str, new_parent_path: &str) -> anyhow::Result<()>;
    async fn copy_file(&self, file_path: &str, new_parent_path: &str) -> anyhow::Result<()>;
    
    // 添加向下转型支持
    fn as_any(&self) -> &dyn std::any::Any;
    
    // 新增：流式下载方法，返回 None 表示不支持流式下载，使用传统下载
    async fn stream_download(&self, _path: &str) -> anyhow::Result<Option<(Box<dyn futures::Stream<Item = Result<axum::body::Bytes, std::io::Error>> + Send + Unpin>, String)>> {
        Ok(None)
    }
    
    // 新增：支持 Range 请求的流式下载方法
    async fn stream_download_with_range(&self, _path: &str, _start: Option<u64>, _end: Option<u64>) -> anyhow::Result<Option<(Box<dyn futures::Stream<Item = Result<axum::body::Bytes, std::io::Error>> + Send + Unpin>, String, u64, Option<u64>)>> {
        Ok(None)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
    pub modified: String,
}

// 驱动配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverInfo {
    pub driver_type: String,
    pub display_name: String,
    pub description: String,
    pub config_schema: serde_json::Value,
}

// 驱动工厂trait
pub trait DriverFactory: Send + Sync {
    fn driver_type(&self) -> &'static str;
    fn driver_info(&self) -> DriverInfo;
    fn create_driver(&self, config: serde_json::Value) -> anyhow::Result<Box<dyn Driver>>;
    fn get_routes(&self) -> Option<axum::Router>;
}

// 全局驱动注册表
static DRIVER_REGISTRY: Lazy<HashMap<String, Box<dyn DriverFactory>>> = Lazy::new(|| {
    let mut registry = HashMap::new();
    
    // 注册本地驱动
    registry.insert("local".to_string(), Box::new(local::LocalDriverFactory) as Box<dyn DriverFactory>);
    
    // 注册OneDrive驱动
    registry.insert("onedrive".to_string(), Box::new(onedrive::OneDriveDriverFactory) as Box<dyn DriverFactory>);
    
    // 注册FTP驱动
    registry.insert("ftp".to_string(), Box::new(ftp::FtpDriverFactory) as Box<dyn DriverFactory>);
    
    // 注册夸克网盘驱动
    registry.insert("quark".to_string(), Box::new(quark::QuarkDriverFactory) as Box<dyn DriverFactory>);
    
    // 注册S3驱动
    registry.insert("s3".to_string(), Box::new(S3DriverFactory) as Box<dyn DriverFactory>);
    
    // 注册天翼云盘驱动
    registry.insert("cloud189".to_string(), Box::new(Cloud189DriverFactory) as Box<dyn DriverFactory>);
    
    // 注册蓝奏云驱动
    registry.insert("lanzou".to_string(), Box::new(LanzouDriverFactory) as Box<dyn DriverFactory>);
    
    // 注册AList驱动
    registry.insert("alist".to_string(), Box::new(AListDriverFactory) as Box<dyn DriverFactory>);
    
    registry
});

// 获取所有可用的驱动信息
pub fn get_available_drivers() -> Vec<DriverInfo> {
    DRIVER_REGISTRY.values().map(|factory| factory.driver_info()).collect()
}

// 根据类型和配置创建驱动
pub fn create_driver(driver_type: &str, config: serde_json::Value) -> anyhow::Result<Box<dyn Driver>> {
    if let Some(factory) = DRIVER_REGISTRY.get(driver_type) {
        factory.create_driver(config)
    } else {
        Err(anyhow::anyhow!("Unknown driver type: {}", driver_type))
    }
}

// 获取所有驱动的路由
pub fn get_all_routes() -> axum::Router {
    let mut router = axum::Router::new();
    
    for factory in DRIVER_REGISTRY.values() {
        if let Some(routes) = factory.get_routes() {
            router = router.merge(routes);
        }
    }
    
    router
}