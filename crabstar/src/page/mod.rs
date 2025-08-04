use crate::fragment::suspense;

pub trait Page {
    fn into_html_stream(
        self,
    ) -> impl ::futures::StreamExt<Item = ::std::result::Result<::std::string::String, suspense::Error>>;
}
