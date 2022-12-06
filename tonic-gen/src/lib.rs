pub mod auth {
    tonic::include_proto!("limit.auth");
}

pub mod event {
    tonic::include_proto!("limit.event");
    pub mod types {
        tonic::include_proto!("limit.event.types");
    }
}

pub mod subs {
    tonic::include_proto!("limit.subs");
    pub mod types {
        tonic::include_proto!("limit.subs.types");
    }
}

pub mod utils {
    tonic::include_proto!("limit.utils");
}
