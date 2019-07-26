use interfacer_http_service::content_type;
use interfacer_http_service::StatusCode;
use proc_macro::{Diagnostic, Level};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, AttrStyle, Attribute, Lit, Meta, MetaList, MetaNameValue,
    NestedMeta, TraitItemMethod,
};

const METHODS: [&'static str; 9] = [
    "get", "post", "put", "delete", "head", "options", "connect", "patch", "trace",
];
const PATH: &'static str = "path";
const CONTENT_TYPE: &'static str = "content_type";
const CHARSET: &'static str = "charset";
const EXPECT: &'static str = "expect";
const STATUS: &'static str = "status";

trait LoadMeta {
    fn load_meta(&mut self, meta: &MetaList) -> Result<(), Diagnostic>;
}

#[derive(Debug)]
struct ContentType {
    content_type: String,
    charset: String,
}

impl Default for ContentType {
    fn default() -> Self {
        Self {
            content_type: content_type::APPLICATION_JSON.into(),
            charset: content_type::CHARSET_UTF8.into(),
        }
    }
}

impl LoadMeta for ContentType {
    fn load_meta(&mut self, meta: &MetaList) -> Result<(), Diagnostic> {
        for nested_meta in meta.nested.iter() {
            if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                ident,
                eq_token: _,
                lit: Lit::Str(token),
            })) = nested_meta
            {
                match ident.to_string().as_str() {
                    CONTENT_TYPE => {
                        self.content_type = token.value();
                    }
                    CHARSET => {
                        self.charset = token.value();
                    }
                    _ => (),
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Expect {
    status: StatusCode,
    content_type: ContentType,
}

impl Default for Expect {
    fn default() -> Self {
        Self {
            status: StatusCode::OK,
            content_type: Default::default(),
        }
    }
}

impl LoadMeta for Expect {
    fn load_meta(&mut self, meta: &MetaList) -> Result<(), Diagnostic> {
        for nested_meta in meta.nested.iter() {
            if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                ident,
                eq_token: _,
                lit: Lit::Int(token),
            })) = nested_meta
            {
                match ident.to_string().as_str() {
                    STATUS => {
                        let status = token.value() as u16;
                        match StatusCode::from_u16(status) {
                            Err(err) => Err(Diagnostic::new(
                                Level::Error,
                                format!("invalid status code: {}", err.to_string()),
                            ))?,
                            Ok(code) => {
                                self.status = code;
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
        self.content_type.load_meta(meta)
    }
}

#[derive(Debug)]
pub struct ReqArgs {
    path: String,
    content_type: ContentType,
}

impl Default for ReqArgs {
    fn default() -> Self {
        Self {
            path: "/".into(),
            content_type: Default::default(),
        }
    }
}

impl LoadMeta for ReqArgs {
    fn load_meta(&mut self, meta: &MetaList) -> Result<(), Diagnostic> {
        for nested_meta in meta.nested.iter() {
            if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                ident,
                eq_token: _,
                lit: Lit::Str(token),
            })) = nested_meta
            {
                match ident.to_string().as_str() {
                    PATH => {
                        self.path = token.value();
                    }
                    _ => (),
                }
            }
        }
        self.content_type.load_meta(meta)
    }
}

struct ArgsTokens {
    req: Option<proc_macro::TokenStream>,
    expect: Option<proc_macro::TokenStream>,
}

#[derive(Default)]
struct Args {
    req: ReqArgs,
    expect: Expect,
}

fn gen_meta(attr: Attribute) -> proc_macro::TokenStream {
    let name = attr.path.segments.last().unwrap().value().ident.clone();
    let tts = attr.tts.clone();
    quote!(#name#tts).into()
}

fn check_duplicate(
    method_name: &str,
    attr: &Option<proc_macro::TokenStream>,
) -> Result<(), Diagnostic> {
    match attr {
        None => Ok(()),
        Some(_) => Err(Diagnostic::new(
            Level::Error,
            format!("method `{}` has duplicate attribute", method_name,),
        )),
    }
}

fn filter_method(raw_method: &TraitItemMethod) -> Result<ArgsTokens, Diagnostic> {
    let method_name = raw_method.sig.ident.to_string();
    let mut tokens = ArgsTokens {
        req: None,
        expect: None,
    };
    for attr in raw_method.attrs.clone() {
        if let AttrStyle::Outer = attr.style {
            let name = attr.path.segments.last().unwrap().value().ident.to_string();
            if name.as_str() == EXPECT {
                check_duplicate(method_name.as_str(), &tokens.expect)?;
                tokens.expect = Some(gen_meta(attr))
            } else if METHODS.contains(&name.as_str()) {
                check_duplicate(method_name.as_str(), &tokens.req)?;
                tokens.req = Some(gen_meta(attr))
            }
        }
    }

    match tokens.req {
        Some(_) => Ok(tokens),
        None => Err(Diagnostic::new(
            Level::Error,
            format!("method `{}` has no request attribute", method_name,),
        )),
    }
}

fn gen_args(req_meta: MetaList, expect_meta: Option<MetaList>) -> Result<Args, Diagnostic> {
    let mut args = Args::default();
    args.req.load_meta(&req_meta)?;
    if let Some(ref meta) = expect_meta {
        args.expect.load_meta(meta)?;
    }
    Ok(args)
}

pub fn transform_method(mut raw_method: TraitItemMethod) -> proc_macro::TokenStream {
    let ArgsTokens { req, expect } = filter_method(&raw_method).unwrap_or_else(|err| {
        err.emit();
        std::process::exit(1);
    });
    let req_token = req.unwrap();
    let req_meta = parse_macro_input!(req_token as MetaList);
    let http_method = req_meta.ident.clone();
    let expect_meta = match expect {
        Some(token) => Some(parse_macro_input!(token as MetaList)),
        None => None,
    };
    let args = gen_args(req_meta, expect_meta).unwrap_or_else(|err| {
        err.emit();
        std::process::exit(1);
    });
    let req_ident = quote!(req);
    let req_define = build_request(&req_ident, http_method.to_string(), &args, &raw_method);
    let body = quote!(
        #req_define
    );
    raw_method.semi_token = None;
    raw_method.default = Some(parse_quote!({
        #body
    }));
    quote!(#raw_method).into()
}

// TODO: complete build request; using generic Body type
fn build_request(
    req_ident: &TokenStream,
    method: String,
    args: &Args,
    _raw_method: &TraitItemMethod,
) -> TokenStream {
    let path = args.req.path.as_str();
    quote!(
        let mut builder = interfacer_http::Request::builder();
        let #req_ident = builder
            .uri(#path)
            .method(#method)
            .body(vec![])?;
    )
}