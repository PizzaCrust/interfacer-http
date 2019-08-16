#![feature(decl_macro, async_await, trait_alias)]

#[doc(inline)]
pub use content::{fail::*, polyfill, FromContent, IntoStruct, ToContent};
#[doc(inline)]
pub use content_type::ContentType;
#[doc(inline)]
pub use fail::{define_from, RequestFail, Result, StringError};
pub use failure::Fail;
#[doc(inline)]
pub use interfacer_http_attribute::http_interface;

pub mod derive {
    #[doc(inline)]
    pub use interfacer_http_attribute::{FromContent, ToContent};
}

#[doc(inline)]
pub use client::{HttpClient, HttpConfig, Response};
pub use interfacer_http_util::*;

mod client;
mod content;
mod content_type;
mod fail;
