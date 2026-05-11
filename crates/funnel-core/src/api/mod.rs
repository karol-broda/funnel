mod types;

pub use types::*;

use std::marker::PhantomData;

use http::Method;

pub struct Endpoint<Req, Resp> {
    pub method: Method,
    pub path: &'static str,
    _marker: PhantomData<fn(Req) -> Resp>,
}

pub const INFO: Endpoint<(), ServerInfo> = Endpoint {
    method: Method::GET,
    path: "/info",
    _marker: PhantomData,
};
