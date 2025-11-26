use miko::dependency_container::LazyDependencyContainer;
use miko::macros::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

static TRANSIENT_CONSTRUCTS: AtomicUsize = AtomicUsize::new(0);
static SINGLETON_CONSTRUCTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Eq)]
struct TransientProbe {
    id: usize,
}

#[component(transient)]
impl TransientProbe {
    async fn new() -> Self {
        let id = TRANSIENT_CONSTRUCTS.fetch_add(1, Ordering::SeqCst) + 1;
        Self { id }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct SingletonProbe {
    id: usize,
}

#[component]
impl SingletonProbe {
    async fn new() -> Self {
        let id = SINGLETON_CONSTRUCTS.fetch_add(1, Ordering::SeqCst) + 1;
        Self { id }
    }
}

#[tokio::test]
async fn component_lifetimes_behave_as_configured() {
    TRANSIENT_CONSTRUCTS.store(0, Ordering::SeqCst);
    SINGLETON_CONSTRUCTS.store(0, Ordering::SeqCst);

    let container = LazyDependencyContainer::new_();
    let guard = container.read().await;

    let transient_a = guard.get::<TransientProbe>().await;
    let transient_b = guard.get::<TransientProbe>().await;
    assert_ne!(Arc::as_ptr(&transient_a), Arc::as_ptr(&transient_b));
    assert_eq!(TRANSIENT_CONSTRUCTS.load(Ordering::SeqCst), 2);

    let singleton_a = guard.get::<SingletonProbe>().await;
    let singleton_b = guard.get::<SingletonProbe>().await;
    assert_eq!(Arc::as_ptr(&singleton_a), Arc::as_ptr(&singleton_b));
    assert_eq!(SINGLETON_CONSTRUCTS.load(Ordering::SeqCst), 1);
}
