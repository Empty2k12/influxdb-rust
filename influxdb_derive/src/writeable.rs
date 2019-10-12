use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
	Fields,
	Ident,
	ItemStruct,
	parse_macro_input
};

pub fn expand_writeable(tokens : TokenStream) -> TokenStream
{
	let input = parse_macro_input!(tokens as ItemStruct);
	let ident = input.ident;
	let generics = input.generics;
	
	let time_field = format_ident!("time");
	let fields : Vec<Ident> = match input.fields {
		Fields::Named(fields) => fields.named.into_iter().map(|field|
			field.ident.expect("fields without ident are not supported")
		).filter(|field| field.to_string() != time_field.to_string()).collect(),
		_ => panic!("a struct without named fields is not supported")
	};
	
	let output = quote! {
		impl #generics ::influxdb::query::InfluxDbWriteable for #ident #generics
		{
			fn into_query(self, name : String) -> ::influxdb::query::write_query::InfluxDbWriteQuery
			{
				let timestamp : ::influxdb::query::Timestamp = self.#time_field;
				let mut query = timestamp.into_query(name);
				#(
					query = query.add_field(stringify!(#fields), &self.#fields);
				)*
				query
			}
		}
	};
	output.into()
}