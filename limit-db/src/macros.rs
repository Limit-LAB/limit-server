/// ```rust
/// let (redis_cluster, redis, db_pool) = get_db_layer!(req);
/// ```
#[macro_export]
macro_rules! get_db_layer {
    ($req:ident) => {
        (
            // TODO: redis_cluster
            (),

            // redis
            $req.extensions()
                .get::<limit_db::RedisClient>()
                .context("no redis extended to service")
                .map_err(|e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                })?
                .clone(),
            
            // db_pool
            $req.extensions()
                .get::<limit_db::DBPool>()
                .context("no db extended to service")
                .map_err(|e| {
                    tracing::error!("{}", e);
                    Status::internal(e.to_string())
                })?
                .clone(),
        )
    };
}
