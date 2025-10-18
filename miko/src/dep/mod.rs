use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

#[cfg(feature = "auto")]
pub static CONTAINER: OnceCell<Arc<RwLock<LazyDependencyContainer>>> = OnceCell::const_new();

pub struct DependencyDefFn(pub fn() -> DependencyDef);
pub struct DependencyDef {
    pub type_id: TypeId,
    pub prewarm: bool,
    pub name: &'static str,
    pub init_fn: fn() -> Pin<Box<dyn Future<Output = Arc<dyn Any + Send + Sync>> + Send>>,
}
#[cfg(feature = "auto")]
inventory::collect!(DependencyDefFn);

type FactoryFuture = Pin<Box<dyn Future<Output = Arc<dyn Any + Send + Sync>> + Send>>;
pub struct LazyDependencyContainer {
    pub registry: HashMap<(TypeId, &'static str), fn() -> FactoryFuture>,
    pub prewarm_flags: HashMap<(TypeId, &'static str), bool>,
    pub instances: HashMap<(TypeId, &'static str), Arc<OnceCell<Arc<dyn Any + Send + Sync>>>>,
}
impl LazyDependencyContainer {
    pub fn new() -> Self {
        LazyDependencyContainer {
            registry: HashMap::new(),
            prewarm_flags: HashMap::new(),
            instances: HashMap::new(),
        }
    }
    #[cfg(feature = "auto")]
    pub fn new_() -> Arc<RwLock<Self>> {
        let mut registry = HashMap::new();
        let mut prewarm_flags = HashMap::new();
        let mut instances = HashMap::new();
        let deps: Vec<DependencyDef> = inventory::iter::<DependencyDefFn>
            .into_iter()
            .map(|v| v.0())
            .collect();

        for dep in deps {
            registry.insert((dep.type_id, "___"), dep.init_fn);
            prewarm_flags.insert((dep.type_id, "___"), dep.prewarm);
            instances.insert((dep.type_id, "___"), Arc::new(OnceCell::new()));
        }

        Arc::new(RwLock::new(Self {
            registry,
            prewarm_flags,
            instances,
        }))
    }
    pub fn register_<T: 'static + Send + Sync>(
        &mut self,
        name: &'static str,
        prewarm: bool,
        factory: fn() -> FactoryFuture,
    ) {
        self.registry.insert((TypeId::of::<T>(), name), factory);
        self.instances
            .insert((TypeId::of::<T>(), name), Arc::new(OnceCell::new()));
        self.prewarm_flags
            .insert((TypeId::of::<T>(), name), prewarm);
    }
    pub fn register<T: 'static + Send + Sync>(
        &mut self,
        prewarm: bool,
        factory: fn() -> FactoryFuture,
    ) {
        self.registry.insert((TypeId::of::<T>(), "___"), factory);
        self.instances
            .insert((TypeId::of::<T>(), "___"), Arc::new(OnceCell::new()));
        self.prewarm_flags
            .insert((TypeId::of::<T>(), "___"), prewarm);
    }
    pub async fn get_<T: 'static + Send + Sync>(&self, name: &'static str) -> Arc<T> {
        let init_fn = self
            .registry
            .get(&(TypeId::of::<T>(), name))
            .expect("No init function found for type");
        let instance = self
            .instances
            .get(&(TypeId::of::<T>(), name))
            .expect("No instance cell found for type");
        let instance = instance.get_or_init(|| init_fn()).await.clone();
        instance.downcast_arc::<T>().unwrap()
    }
    pub async fn get<T: 'static + Send + Sync>(&self) -> Arc<T> {
        let init_fn = self
            .registry
            .get(&(TypeId::of::<T>(), "___"))
            .expect("No init function found for type");
        let instance = self
            .instances
            .get(&(TypeId::of::<T>(), "___"))
            .expect("No instance cell found for type");
        let instance = instance.get_or_init(|| init_fn()).await.clone();
        instance.downcast_arc::<T>().unwrap()
    }

    pub async fn prewarm_all(&self) {
        for (type_id, prewarm) in &self.prewarm_flags {
            if *prewarm {
                let init_fn = self.registry.get(type_id).unwrap();
                let cell = self.instances.get(type_id).unwrap();
                let _ = cell.get_or_init(|| init_fn()).await;
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
