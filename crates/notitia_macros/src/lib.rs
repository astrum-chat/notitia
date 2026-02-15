use proc_macro::TokenStream;

mod database;
use database::impl_database;

mod record;
use record::impl_record;

mod utils;

#[proc_macro_attribute]
pub fn database(args: TokenStream, item: TokenStream) -> TokenStream {
    impl_database(args, item)
}

#[proc_macro_attribute]
pub fn record(args: TokenStream, item: TokenStream) -> TokenStream {
    impl_record(args, item)
}
