use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::OnceCell;
#[cfg(feature = "auto")]
use tokio::sync::{RwLock, RwLockReadGuard};

#[cfg(feature = "auto")]
pub static CONTAINER: OnceCell<Arc<RwLock<LazyDependencyContainer>>> = OnceCell::const_new();

type DependencyInstanceFuture = Pin<Box<dyn Future<Output = DependencyInstance> + Send>>;
type DependencyInstance = Arc<dyn Any + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyLifetime {
    Singleton,
    Transient,
}

pub struct DependencyDefFn(pub fn() -> DependencyDef);
pub struct DependencyDef {
    pub type_id: TypeId,
    pub prewarm: bool,
    pub name: &'static str,
    pub init_fn: fn() -> DependencyInstanceFuture,
    pub lifetime: DependencyLifetime,
}
#[cfg(feature = "auto")]
inventory::collect!(DependencyDefFn);

type FactoryFuture = Pin<Box<dyn Future<Output = DependencyInstance> + Send>>;

#[derive(Clone)]
pub struct DependencyEntry {
    factory: fn() -> FactoryFuture,
    lifetime: DependencyLifetime,
    prewarm: bool,
    instance: Option<Arc<OnceCell<DependencyInstance>>>,
}

impl DependencyEntry {
    fn new(factory: fn() -> FactoryFuture, lifetime: DependencyLifetime, prewarm: bool) -> Self {
        let instance = if matches!(lifetime, DependencyLifetime::Singleton) {
            Some(Arc::new(OnceCell::new()))
        } else {
            None
        };
        Self {
            factory,
            lifetime,
            prewarm,
            instance,
        }
    }
}

pub struct LazyDependencyContainer {
    pub registry: HashMap<(TypeId, &'static str), DependencyEntry>,
}
impl LazyDependencyContainer {
    /// 创建一个新的依赖容器
    pub fn new() -> Self {
        LazyDependencyContainer {
            registry: HashMap::new(),
        }
    }
    #[cfg(feature = "auto")]
    /// 创建一个新的依赖容器,并自动收集所有注册的依赖
    pub fn new_() -> Arc<RwLock<Self>> {
        let mut registry = HashMap::new();
        let deps: Vec<DependencyDef> = inventory::iter::<DependencyDefFn>
            .into_iter()
            .map(|v| v.0())
            .collect();

        for dep in deps {
            registry.insert(
                (dep.type_id, dep.name),
                DependencyEntry::new(dep.init_fn, dep.lifetime, dep.prewarm),
            );
        }

        Arc::new(RwLock::new(Self { registry }))
    }

    fn insert_entry<T: 'static + Send + Sync>(
        &mut self,
        name: &'static str,
        prewarm: bool,
        lifetime: DependencyLifetime,
        factory: fn() -> FactoryFuture,
    ) {
        self.registry.insert(
            (TypeId::of::<T>(), name),
            DependencyEntry::new(factory, lifetime, prewarm),
        );
    }

    pub fn register_with_lifetime_<T: 'static + Send + Sync>(
        &mut self,
        name: &'static str,
        prewarm: bool,
        lifetime: DependencyLifetime,
        factory: fn() -> FactoryFuture,
    ) {
        self.insert_entry::<T>(name, prewarm, lifetime, factory);
    }

    pub fn register_with_lifetime<T: 'static + Send + Sync>(
        &mut self,
        prewarm: bool,
        lifetime: DependencyLifetime,
        factory: fn() -> FactoryFuture,
    ) {
        self.insert_entry::<T>("___", prewarm, lifetime, factory);
    }

    pub fn register_<T: 'static + Send + Sync>(
        &mut self,
        name: &'static str,
        prewarm: bool,
        factory: fn() -> FactoryFuture,
    ) {
        self.register_with_lifetime_::<T>(name, prewarm, DependencyLifetime::Singleton, factory);
    }
    pub fn register<T: 'static + Send + Sync>(
        &mut self,
        prewarm: bool,
        factory: fn() -> FactoryFuture,
    ) {
        self.register_with_lifetime::<T>(prewarm, DependencyLifetime::Singleton, factory);
    }
    pub async fn get_<T: 'static + Send + Sync>(&self, name: &'static str) -> Arc<T> {
        let entry = self
            .registry
            .get(&(TypeId::of::<T>(), name))
            .expect("No dependency entry found for type");
        self.resolve_entry(entry).await.downcast_arc::<T>().unwrap()
    }
    pub async fn get<T: 'static + Send + Sync>(&self) -> Arc<T> {
        let entry = self
            .registry
            .get(&(TypeId::of::<T>(), "___"))
            .expect("No dependency entry found for type");
        self.resolve_entry(entry).await.downcast_arc::<T>().unwrap()
    }

    async fn resolve_entry(&self, entry: &DependencyEntry) -> DependencyInstance {
        match entry.lifetime {
            DependencyLifetime::Singleton => {
                let cell = entry
                    .instance
                    .as_ref()
                    .expect("Singleton dependency missing storage cell");
                cell.get_or_init(entry.factory).await.clone()
            }
            DependencyLifetime::Transient => (entry.factory)().await,
        }
    }

    pub async fn prewarm_all(&self) {
        for entry in self.registry.values() {
            #[allow(clippy::collapsible_if)]
            if entry.prewarm && entry.lifetime == DependencyLifetime::Singleton {
                if let Some(cell) = &entry.instance {
                    let _ = cell.get_or_init(entry.factory).await;
                }
            }
        }
    }
}

pub trait ArcAnyExt {
    fn downcast_arc<T: Any + Send + Sync>(self: Arc<Self>) -> Option<Arc<T>>
    where
        Self: Send + Sync + 'static;
}

impl ArcAnyExt for dyn Any + Send + Sync {
    fn downcast_arc<T: Any + Send + Sync>(self: Arc<Self>) -> Option<Arc<T>>
    where
        Self: Send + Sync + 'static,
    {
        if self.is::<T>() {
            let raw = Arc::into_raw(self) as *const T;
            Some(unsafe { Arc::from_raw(raw) })
        } else {
            None
        }
    }
}

/// 获取全局的依赖容器
#[cfg(feature = "auto")]
pub async fn get_global_dc() -> RwLockReadGuard<'static, LazyDependencyContainer> {
    CONTAINER.get().unwrap().read().await
}
