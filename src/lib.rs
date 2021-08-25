//! Generate TypeScript type definitions for Rust types.
//!
//! This crate allows you to produce a TypeScript module containing type
//! definitions which describe the JSON serialization of Rust types. The
//! intended use is to define TypeScript types for data that is serialized from
//! Rust types as JSON using [`serde_json`](https://docs.rs/serde_json/) so it
//! can be safely used from TypeScript without needing to maintain a parallel
//! set of type definitions.
//!
//! One example of where this crate is useful is when working on a web
//! application project with a Rust backend and a TypeScript frontend. If the
//! data used to communicate between the two is defined in Rust and uses
//! [`serde_json`](https://docs.rs/serde_json/) to encode/decode it for
//! transmission across the network, you can use this crate to automatically
//! generate a TypeScript definition file for those types in order to use them
//! safely in your frontend code. This process can even be completely automated
//! if you use this crate in a
//! [build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
//! for your server to write the definition file to your TypeScript source code
//! directory.
//!
//! # Examples
//!
//! Simple example:
//! ```
//! use serde::Serialize;
//! use typescript_type_def::{write_definition_file, TypeDef};
//!
//! #[derive(Serialize, TypeDef)]
//! struct Foo {
//!     a: usize,
//!     b: String,
//! }
//!
//! let ts_module = {
//!     let mut buf = Vec::new();
//!     write_definition_file::<_, Foo>(&mut buf, Default::default()).unwrap();
//!     String::from_utf8(buf).unwrap()
//! };
//! assert_eq!(
//!     ts_module,
//!     r#"// AUTO-GENERATED by typescript-type-def
//!
//! export default types;
//! export namespace types{
//! export type Usize=number;
//! export type Foo={"a":types.Usize;"b":string;};
//! }
//! "#
//! );
//!
//! let foo = Foo {
//!     a: 123,
//!     b: "hello".to_owned(),
//! };
//! let json = serde_json::to_string(&foo).unwrap();
//! // This JSON matches the TypeScript type definition above
//! assert_eq!(json, r#"{"a":123,"b":"hello"}"#);
//! ```
//!
//! When working with a large codebase consisting of many types, a useful
//! pattern is to declare an "API" type alias which lists all the types you want
//! to make definitions for. For example:
//! ```
//! use serde::Serialize;
//! use typescript_type_def::{write_definition_file, TypeDef};
//!
//! #[derive(Serialize, TypeDef)]
//! struct Foo {
//!     a: String,
//! }
//!
//! #[derive(Serialize, TypeDef)]
//! struct Bar {
//!     a: String,
//! }
//!
//! #[derive(Serialize, TypeDef)]
//! struct Baz {
//!     a: Qux,
//! }
//!
//! #[derive(Serialize, TypeDef)]
//! struct Qux {
//!     a: String,
//! }
//!
//! // This type lists all the top-level types we want to make definitions for.
//! // You don't need to list *every* type in your API here, only ones that
//! // wouldn't be referenced otherwise. Note that `Qux` is not mentioned, but
//! // is still emitted because it is a dependency of `Baz`.
//! type Api = (Foo, Bar, Baz);
//!
//! let ts_module = {
//!     let mut buf = Vec::new();
//!     write_definition_file::<_, Api>(&mut buf, Default::default()).unwrap();
//!     String::from_utf8(buf).unwrap()
//! };
//! assert_eq!(
//!     ts_module,
//!     r#"// AUTO-GENERATED by typescript-type-def
//!
//! export default types;
//! export namespace types{
//! export type Foo={"a":string;};
//! export type Bar={"a":string;};
//! export type Qux={"a":string;};
//! export type Baz={"a":types.Qux;};
//! }
//! "#
//! );
//! ```
#![warn(rust_2018_idioms, clippy::all, missing_docs)]
#![deny(clippy::correctness)]

mod emit;
mod impls;
mod iter_refs;
pub mod type_expr;

pub use crate::{
    emit::{write_definition_file, DefinitionFileOptions, Stats, TypeDef},
    impls::Blob,
};

/// A derive proc-macro for the [`TypeDef`] trait.
///
/// This macro can be used on `struct`s and `enum`s which also derive
/// [`serde::Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html)
/// and/or
/// [`serde::Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html),
/// and will generate a [`TypeDef`] implementation which matches the shape
/// of the JSON produced by using [`serde_json`](https://docs.rs/serde_json/) on
/// the target type. This macro will also read and adapt to `#[serde(...)]`
/// attributes on the target type's definition.
///
/// This macro also reads the following attributes:
/// * `#[type_def(namespace = "x.y.z")]` on the `struct`/`enum` body puts
///   the TypeScript type definition under a namespace of `x.y.z`. Note
///   that [`write_definition_file`] will additionally place all type
///   definitions under a root namespace (by default named `types`).
///
/// ## `serde` attribute support
///
/// Legend:
/// * ✓ - full support
/// * ? - may support in the future
/// * ✗ - will not support
///
/// ### Container Attributes
/// | Attribute | Support |
/// |:-|:-:|
/// | [`#[serde(rename = "name")]`](https://serde.rs/container-attrs.html#rename) | ✓ |
/// | [`#[serde(rename_all = "...")]`](https://serde.rs/container-attrs.html#rename_all) | ✓ |
/// | [`#[serde(deny_unknown_fields)]`](https://serde.rs/container-attrs.html#deny_unknown_fields) | ? |
/// | [`#[serde(tag = "type")]`](https://serde.rs/container-attrs.html#tag) | ✓ |
/// | [`#[serde(tag = "t", content = "c")]`](https://serde.rs/container-attrs.html#tag--content) | ✓ |
/// | [`#[serde(untagged)]`](https://serde.rs/container-attrs.html#untagged) | ✓ |
/// | [`#[serde(bound = "T: MyTrait")]`](https://serde.rs/container-attrs.html#bound) | ? |
/// | [`#[serde(default)]`](https://serde.rs/container-attrs.html#default) | ? |
/// | [`#[serde(default = "path")]`](https://serde.rs/container-attrs.html#default--path) | ? |
/// | [`#[serde(remote = "...")]`](https://serde.rs/container-attrs.html#remote) | ✗ |
/// | [`#[serde(transparent)]`](https://serde.rs/container-attrs.html#transparent) | ✓ |
/// | [`#[serde(from = "FromType")]`](https://serde.rs/container-attrs.html#from) | ✗ |
/// | [`#[serde(try_from = "FromType")]`](https://serde.rs/container-attrs.html#try_from) | ✗ |
/// | [`#[serde(into = "IntoType")]`](https://serde.rs/container-attrs.html#into) | ✗ |
/// | [`#[serde(crate = "...")]`](https://serde.rs/container-attrs.html#crate) | ✗ |
///
/// ### Variant Attributes
/// | Attribute | Support |
/// |:-|:-:|
/// | [`#[serde(rename = "name")]`](https://serde.rs/variant-attrs.html#rename) | ✓ |
/// | [`#[serde(alias = "name")]`](https://serde.rs/variant-attrs.html#alias) | ? |
/// | [`#[serde(rename_all = "...")]`](https://serde.rs/variant-attrs.html#rename_all) | ✓ |
/// | [`#[serde(skip)]`](https://serde.rs/variant-attrs.html#skip) | ✓ |
/// | [`#[serde(skip_serializing)]`](https://serde.rs/variant-attrs.html#skip_serializing) | ✗ |
/// | [`#[serde(skip_deserializing)]`](https://serde.rs/variant-attrs.html#skip_deserializing) | ✗ |
/// | [`#[serde(serialize_with = "path")]`](https://serde.rs/variant-attrs.html#serialize_with) | ✗ |
/// | [`#[serde(deserialize_with = "path")]`](https://serde.rs/variant-attrs.html#deserialize_with) | ✗ |
/// | [`#[serde(with = "module")]`](https://serde.rs/variant-attrs.html#with) | ✗ |
/// | [`#[serde(bound = "T: MyTrait")]`](https://serde.rs/variant-attrs.html#bound) | ? |
/// | [`#[serde(borrow)]`](https://serde.rs/variant-attrs.html#borrow) | ? |
/// | [`#[serde(borrow = "'a + 'b + ...")]`](https://serde.rs/variant-attrs.html#borrow) | ? |
/// | [`#[serde(other)]`](https://serde.rs/variant-attrs.html#other) | ? |
///
/// ### Field Attributes
/// | Attribute | Support |
/// |:-|:-:|
/// | [`#[serde(rename = "name")]`](https://serde.rs/field-attrs.html#rename) | ✓ |
/// | [`#[serde(alias = "name")]`](https://serde.rs/field-attrs.html#alias) | ? |
/// | [`#[serde(default)]`](https://serde.rs/field-attrs.html#default) | ✓ |
/// | [`#[serde(default = "path")]`](https://serde.rs/field-attrs.html#default--path) | ? |
/// | [`#[serde(flatten)]`](https://serde.rs/field-attrs.html#flatten) | ✓ |
/// | [`#[serde(skip)]`](https://serde.rs/field-attrs.html#skip) | ✓ |
/// | [`#[serde(skip_serializing)]`](https://serde.rs/field-attrs.html#skip_serializing) | ✗ |
/// | [`#[serde(skip_deserializing)]`](https://serde.rs/field-attrs.html#skip_deserializing) | ✗ |
/// | [`#[serde(skip_serializing_if = "path")]`](https://serde.rs/field-attrs.html#skip_serializing_if) | ✓ |
/// | [`#[serde(serialize_with = "path")]`](https://serde.rs/field-attrs.html#serialize_with) | ✗ |
/// | [`#[serde(deserialize_with = "path")]`](https://serde.rs/field-attrs.html#deserialize_with) | ✗ |
/// | [`#[serde(with = "module")]`](https://serde.rs/field-attrs.html#with) | ✗ |
/// | [`#[serde(borrow)]`](https://serde.rs/field-attrs.html#borrow) | ? |
/// | [`#[serde(borrow = "'a + 'b + ...")]`](https://serde.rs/field-attrs.html#borrow) | ? |
/// | [`#[serde(bound = "T: MyTrait")]`](https://serde.rs/field-attrs.html#bound) | ? |
/// | [`#[serde(getter = "...")]`](https://serde.rs/field-attrs.html#getter) | ✗ |
pub use typescript_type_def_derive::TypeDef;
