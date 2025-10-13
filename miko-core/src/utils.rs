use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

const SAFE_CHARS: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b'-')
    .remove(b'_')
    .remove(b':')
    .remove(b'{')
    .remove(b'}');

pub fn encode_route(path: &str) -> String {
  utf8_percent_encode(path, SAFE_CHARS).to_string()
}

pub fn decode_path(path: &str) -> String {
  percent_encoding::percent_decode_str(path).decode_utf8_lossy().trim_start_matches('/').to_string()
}