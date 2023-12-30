//! This crate allows defining arbitrary aliases for attributes.
//!
//! Aliases are resolved by [`#[attr_alias]`][macro@attr_alias]. Since that
//! attribute requires a nightly compliler, [`#[eval]`][macro@eval] and
//! [`eval_block!`] provide workarounds for use on the stable release channel.
//!
//! # Alias File
//!
//! Due to how procedural macros work and to avoid redundancy, this crate will
//! always read aliases from
#![doc = concat!("\"", alias_file!(), "\".")]
//! Other files may be supported in future versions, but doing so is not
//! currently possible. Open an issue if this is important for your build.
//!
//! ## Syntax
//!
//! - Each alias must begin with `*` and be assigned to a valid attribute
//!   value.
//! - Aliases can reference others, but referenced aliases must be listed
//!   first.
//!
//! ## Example
//!
//! ```ignore
#![doc = include_str!(concat!("../", alias_file!()))]
//! ```
//!
//! # Features
//!
//! These features are optional and can be enabled or disabled in a
//! "Cargo.toml" file.
//!
//! ### Nightly Features
//!
//! These features are unstable, since they rely on unstable Rust features.
//!
//! - **nightly** -
//!   Provides [`#[attr_alias]`][macro@attr_alias].
//!
//! # Dependencies
//!
//! Although this is a proc\_macro crate, it does not depend on [proc\_macro2],
//! [quote], or [syn]. Therefore, its impact on compile time should be minimal.
//!
//! # Comparable Crates
//!
//! The following crates are similar but take different approaches. An overview
//! of benefits and downsides in comparison to this crate is provided for each
//! when expanded.
//!
//! <ul><li><details><summary>
//!
//! **[cfg\_aliases]** -
//! Aliases defined using "build.rs" instructions.
//!
//! </summary>
//!
//! - *Pros:*
//!     - Compile time may be reduced. The declarative macro is only used in
//!       the build file, but the build file must be compiled as well.
//!     - Inner attributes are supported without a nightly feature.
//! - *Cons:*
//!     - Only `#[cfg]` aliases can be defined.
//!     - Some configuration options are not supported (e.g., `test`).
//!     - Alias names are not checked at compile time.
//!     - Aliases are not expanded inline, as would be desirable for
//!       `#[doc(cfg)]`.
//!
//! </details></li><li><details><summary>
//!
//! **[macro\_rules\_attribute]** -
//! Aliases defined as declarative macros.
//!
//! </summary>
//!
//! - *Pros:*
//!     - Aliases are defined within Rust source files.
//!     - Aliases can expand to multiple attributes.
//!     - Declarative macros accepting valid Rust syntax can be used as
//!       attributes.
//! - *Cons:*
//!     - Attributes cannot be attached to statements without a nightly
//!       feature.
//!     - Inner attributes are not supported.
//!     - Aliases cannot be inserted at a specific part of an attribute
//!       (e.g., within `not()`).
//!     - Some dependencies are required, which may impact compile time.
//!
//! </details></li></ul>
//!
//! [cfg\_aliases]: https://crates.io/crates/cfg_aliases
//! [macro\_rules\_attribute]: https://crates.io/crates/macro_rules_attribute
//! [proc\_macro2]: https://crates.io/crates/proc_macro2
//! [quote]: https://crates.io/crates/quote
//! [syn]: https://crates.io/crates/syn

// Only require a nightly compiler when building documentation for docs.rs.
// This is a private option that should not be used.
// https://github.com/rust-lang/docs.rs/issues/147#issuecomment-389544407
#![cfg_attr(attr_alias_docs_rs, feature(doc_cfg))]
#![cfg_attr(feature = "nightly", feature(track_path))]
#![forbid(unsafe_code)]
#![warn(unused_results)]

use std::error;
use std::result;

#[cfg(feature = "nightly")]
use proc_macro::tracked_path;
use proc_macro::Delimiter;
use proc_macro::Group;
use proc_macro::Literal;
use proc_macro::Punct;
use proc_macro::Spacing;
use proc_macro::Span;
use proc_macro::TokenStream;
use proc_macro::TokenTree;

macro_rules! alias_file {
    () => {
        "src/attr-aliases.txt"
    };
}

macro_rules! tokens {
    ( $($token:expr ,)+ ) => {{
        use proc_macro::TokenTree;

        [$(TokenTree::from($token)),+].into_iter()
    }};
}

macro_rules! path {
    ( $($name:expr),+ ) => {{
        use proc_macro::Ident;
        use proc_macro::Punct;
        use proc_macro::Spacing;
        use proc_macro::Span;

        tokens!(
            $(
                Punct::new(':', Spacing::Joint),
                Punct::new(':', Spacing::Alone),
                Ident::new($name, Span::call_site()),
            )+
        )
    }};
}

mod aliases;
use aliases::Aliases;

fn core_macro(name: &str, arg: &str) -> impl Iterator<Item = TokenTree> {
    path!("core", name).chain(tokens!(
        Punct::new('!', Spacing::Alone),
        Group::new(
            Delimiter::Parenthesis,
            TokenTree::Literal(Literal::string(arg)).into(),
        ),
        Punct::new(';', Spacing::Alone),
    ))
}

struct Error {
    span: Span,
    message: String,
}

impl Error {
    fn new(message: &'static str) -> Self {
        Self {
            span: Span::call_site(),
            message: message.to_owned(),
        }
    }

    fn new_from<T>(error: T, message: &'static str) -> Self
    where
        T: error::Error,
    {
        Self {
            span: Span::call_site(),
            message: format!("error {}: {}", message, error),
        }
    }

    fn token(token: &TokenTree) -> Self {
        Self {
            span: token.span(),
            message: "unexpected token".to_owned(),
        }
    }

    fn into_compile_error(self) -> TokenStream {
        core_macro("compile_error", &self.message)
            .map(|mut token| {
                token.set_span(self.span);
                token
            })
            .collect()
    }
}

fn parse_empty<I>(tokens: I) -> Result<()>
where
    I: IntoIterator<Item = TokenTree>,
{
    tokens
        .into_iter()
        .next()
        .map(|x| Err(Error::token(&x)))
        .unwrap_or(Ok(()))
}

type Result<T> = result::Result<T, Error>;

fn eval_item(item: TokenStream, resolved: &mut bool) -> Result<TokenStream> {
    let mut attr = false;
    item.into_iter()
        .map(|mut token| {
            if let TokenTree::Group(group) = &mut token {
                let delimiter = group.delimiter();
                let mut stream = group.stream();
                if attr && delimiter == Delimiter::Bracket {
                    *resolved |= Aliases::get()?.resolve(&mut stream)?;
                } else {
                    stream = eval_item(stream, resolved)?;
                };
                *group = Group::new(delimiter, stream);
            }
            attr = matches!(
                &token,
                TokenTree::Punct(x)
                    if x.as_char() == '#' || (attr && x.as_char() == '!'),
            );
            Ok(token)
        })
        .collect()
}

/// Resolves an alias using a pattern.
///
/// # Arguments
///
/// The following positional arguments are expected:
/// 1. *alias name* - required and must be a valid [Rust identifier]
/// 2. *expansion pattern* - optional and may include `*` wildcards
///     - The first wildcard in this pattern will be replaced with the expanded
///       alias.
///     - If not specified, this argument defaults to the value of the
///       "default" alias, or `*` if that alias is not defined.
///
/// For example, using the [example alias file], the annotations
/// `#[attr_alias(macos, cfg(*))]` and `#[attr_alias(macos)]` would both expand
/// to `#[cfg(target_os = "macos")]`.
///
/// # Examples
///
/// *Compiled using the [example alias file].*
///
/// ```
/// use std::process::Command;
///
/// use attr_alias::attr_alias;
///
/// struct ProcessBuilder(Command);
///
/// impl ProcessBuilder {
///     #[attr_alias(macos_or_windows)]
#[cfg_attr(
    attr_alias_docs_rs,
    doc = "    #[attr_alias(macos_or_windows, doc(cfg(*)))]"
)]
///     fn name(&mut self, name: &str) -> &mut Self {
///         unimplemented!();
///     }
/// }
/// ```
///
/// [example alias file]: self#example
/// [Rust identifier]: https://doc.rust-lang.org/reference/identifiers.html
#[cfg(feature = "nightly")]
#[cfg_attr(attr_alias_docs_rs, doc(cfg(feature = "nightly")))]
#[proc_macro_attribute]
pub fn attr_alias(args: TokenStream, item: TokenStream) -> TokenStream {
    tracked_path::path(Aliases::FILE);

    Aliases::get()
        .and_then(|x| x.resolve_args(args))
        .map(|alias| {
            tokens!(
                Punct::new('#', Spacing::Joint),
                Group::new(Delimiter::Bracket, alias),
            )
            .chain(item)
            .collect()
        })
        .unwrap_or_else(Error::into_compile_error)
}

/// Equivalent to [`#[eval]`][macro@eval] but does not have restrictions on
/// where it can be attached.
///
/// # Examples
///
/// *Compiled using the [example alias file].*
///
/// Non-inline modules can be annotated:
///
/// ```
/// attr_alias::eval_block! {
///     #[attr_alias(macos, cfg_attr(*, path = "sys/macos.rs"))]
///     #[attr_alias(macos, cfg_attr(not(*), path = "sys/common.rs"))]
///     mod sys;
/// }
/// ```
#[cfg_attr(
    feature = "nightly",
    doc = "
Using [`#[eval]`][macro@eval] would require a nightly feature:

```
#![feature(proc_macro_hygiene)]

#[attr_alias::eval]
#[attr_alias(macos, cfg_attr(*, path = \"sys/macos.rs\"))]
#[attr_alias(macos, cfg_attr(not(*), path = \"sys/common.rs\"))]
mod sys;
```"
)]
///
/// [example alias file]: self#example
#[proc_macro]
pub fn eval_block(item: TokenStream) -> TokenStream {
    let mut resolved = false;
    let mut result = eval_item(item, &mut resolved)
        .unwrap_or_else(Error::into_compile_error);

    let trigger = if resolved {
        Aliases::create_trigger()
    } else {
        Err(Error::new("unnecessary attribute"))
    };
    match trigger {
        Ok(trigger) => result.extend(trigger),
        Err(error) => result.extend(error.into_compile_error()),
    }

    result
}

/// Resolves [`#[attr_alias]`][macro@attr_alias] attributes.
///
/// This attribute must be attached to a file-level item. It allows
/// [`#[attr_alias]`][macro@attr_alias] attributes within that item to be
/// resolved without nightly features.
///
/// # Errors
///
/// Errors will typically be clear, but for those that are not, they can be
/// interpreted as follows:
/// - *"cannot find attribute `attr_alias` in this scope"* -
///   The [`#[attr_alias]`][macro@attr_alias] attribute was used without this
///   attribute or importing it.
/// - *"`const` items in this context need a name"* -
///   This attribute was attached to an item that is not at the top level of a
///   file.
/// - *"non-inline modules in proc macro input are unstable"* ([E0658]) -
///   Due to the [proc\_macro\_hygiene] feature being unstable, [`eval_block!`]
///   should be used instead.
///
/// # Examples
///
/// *Compiled using the [example alias file].*
///
/// **Conditionally Defining a Method:**
///
/// ```
/// use std::process::Command;
///
/// struct ProcessBuilder(Command);
///
/// #[attr_alias::eval]
/// impl ProcessBuilder {
///     #[attr_alias(macos_or_windows)]
#[cfg_attr(
    attr_alias_docs_rs,
    doc = "    #[attr_alias(macos_or_windows, doc(cfg(*)))]"
)]
///     fn name(&mut self, name: &str) -> &mut Self {
///         unimplemented!();
///     }
/// }
/// ```
#[cfg_attr(
    feature = "nightly",
    doc = "
**Setting Lint Configuration:**

```
#![feature(custom_inner_attributes)]
# #![feature(prelude_import)]

#![attr_alias::eval]
#![attr_alias(warnings, *)]
```"
)]
///
/// [E0658]: https://doc.rust-lang.org/error_codes/E0658.html
/// [example alias file]: self#example
/// [proc\_macro\_hygiene]: https://doc.rust-lang.org/unstable-book/language-features/proc-macro-hygiene.html
#[proc_macro_attribute]
pub fn eval(args: TokenStream, item: TokenStream) -> TokenStream {
    if let Err(error) = parse_empty(args) {
        return error.into_compile_error();
    }

    eval_block(item)
}
