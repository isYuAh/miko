mod route;
pub use route::*;

/// 初始化依赖容器，注册并后台预热所有组件
pub async fn init_container() {
    crate::dependency_container::CONTAINER
        .get_or_init(|| async { crate::dependency_container::LazyDependencyContainer::new_() })
        .await;
    tokio::spawn(async {
        crate::dependency_container::CONTAINER
            .get()
            .unwrap()
            .read()
            .await
            .prewarm_all()
            .await;
    });
}
