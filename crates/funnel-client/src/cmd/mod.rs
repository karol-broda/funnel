macro_rules! examples {
    ($($example:expr),+ $(,)?) => {
        concat!($("  ", $example, "\n",)+)
    };
}
pub(crate) use examples;

pub mod api;
pub mod auth;
pub mod cli_reference;
pub mod config;
pub mod context;
pub mod http;
pub mod keys;
pub mod sessions;
pub mod status;
pub mod tcp;
pub mod teams;
pub mod tls;
pub mod users;
pub mod whoami;
