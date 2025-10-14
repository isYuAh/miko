use crate::handler::router::Router;

pub struct RouteFlag {
    pub register: fn(Router) -> Router,
}

inventory::collect!(RouteFlag);

pub fn collect_global_router() -> Router {
    let mut router = Router::new();
    for flag in inventory::iter::<RouteFlag> {
        router = (flag.register)(router);
    }
    router
}
