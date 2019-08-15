#[allow(unused_imports)]
use super::encoding::disable_encoding_error;

#[cfg(feature = "encoding")]
use super::encoding::decode_data;

use super::fail::FromContentFail;
use crate::content_type::ContentType;
use crate::content_types::*;
use crate::fail::StringError;
use crate::polyfill::FromContentHtml;
use unhtml::FromHtml;

impl<T: FromHtml> FromContentHtml for T {
    type Err = FromContentFail;
    fn _from_content(data: Vec<u8>, content_type: &ContentType) -> Result<Self, Self::Err> {
        match content_type.base_type() {
            TEXT_HTML => match content_type.encoding() {
                None | Some(CHARSET_UTF8) => Ok(T::from_html(&String::from_utf8(data)?)?),
                #[cfg(feature = "encoding")]
                Some(encoding) => Ok(T::from_html(&decode_data(&data, encoding)?)?),
                #[cfg(not(feature = "encoding"))]
                Some(encoding) => Err(disable_encoding_error(encoding).into()),
            },
            unsupported => {
                Err(StringError::new(format!("unsupported content type '{}'", unsupported)).into())
            }
        }
    }
}
