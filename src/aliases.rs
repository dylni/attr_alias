use std::collections::HashMap;
use std::env;
use std::fs::OpenOptions;
use std::io::Read;
use std::sync::OnceLock;

use proc_macro::Delimiter;
use proc_macro::Group;
use proc_macro::Ident;
use proc_macro::Punct;
use proc_macro::Spacing;
use proc_macro::Span;
use proc_macro::TokenStream;
use proc_macro::TokenTree;

use super::Error;
use super::Result;

macro_rules! next {
    ( $item:expr , $type:ident $(, $method:ident => $value:expr)? $(,)? ) => {
        if let Some(token) = $item.next() {
            match token {
                TokenTree::$type(x) $(if x.$method() == $value)? => Ok(x),
                _ => Err(Error::token(&token)),
            }
        } else {
            Err(Error::new("unexpected end of tokens"))
        }
    }
}

fn is_comma(token: &TokenTree) -> bool {
    matches!(token, TokenTree::Punct(x) if x.as_char() == ',')
}

pub(super) struct Aliases(HashMap<String, String>);

impl Aliases {
    pub(super) const FILE: &'static str = alias_file!();

    pub(super) fn resolve_args(
        &self,
        args: TokenStream,
    ) -> Result<TokenStream> {
        const DEFAULT_NAME: &str = "default";

        let mut args = args.into_iter().fuse();
        let name = next!(args, Ident)?;
        let mut pattern = args
            .next()
            .map(|token| {
                if !is_comma(&token) {
                    return Err(Error::token(&token));
                }

                let pattern: TokenStream =
                    args.by_ref().take_while(|x| !is_comma(x)).collect();
                super::parse_empty(args)?;
                Ok(pattern)
            })
            .transpose()?
            .filter(|x| !x.is_empty());

        // The default alias does not make sense to nest, as the only way to
        // nest it would be to nest [#[attr_alias]], which already has syntax
        // for it to be implicitly used.
        let alias = Some(name.to_string())
            .filter(|x| x != DEFAULT_NAME)
            .and_then(|x| self.0.get(&x))
            .ok_or_else(|| Error {
                span: name.span(),
                message: format!("unknown alias '{}'", name),
            })?;
        if let Some(pattern) = &mut pattern {
            let _ = self.resolve(pattern)?;
        }
        Ok(pattern
            .map(|x| x.to_string())
            .as_ref()
            .or_else(|| self.0.get(DEFAULT_NAME))
            .map(|x| x.replacen('*', alias, 1))
            .as_ref()
            .unwrap_or(alias)
            .parse()
            .expect("error parsing alias"))
    }

    pub(super) fn resolve(&self, attr: &mut TokenStream) -> Result<bool> {
        let mut attr_iter = attr.clone().into_iter();
        next!(attr_iter, Ident, to_string => "attr_alias")
            .ok()
            .map(|_| {
                let args = next!(
                    attr_iter,
                    Group,
                    delimiter => Delimiter::Parenthesis,
                )?;
                super::parse_empty(attr_iter)?;
                Ok(args.stream())
            })
            .transpose()?
            .map(|args| self.resolve_args(args).map(|x| *attr = x))
            .transpose()
            .map(|x| x.is_some())
    }

    fn parse() -> Result<Self> {
        let mut aliases = "\n".to_owned();
        let _ = OpenOptions::new()
            .read(true)
            .open(Self::FILE)
            .map_err(|x| Error::new_from(x, "opening alias file"))?
            .read_to_string(&mut aliases)
            .map_err(|x| Error::new_from(x, "reading alias file"))?;

        let mut parsed_aliases = Self(HashMap::new());
        let mut aliases = aliases.split("\n*").peekable();
        let _ = aliases.next_if_eq(&"");
        for alias in aliases {
            let mut alias = alias
                .parse::<TokenStream>()
                .map_err(|x| Error::new_from(x, "parsing alias file"))?
                .into_iter();
            let alias_name = next!(alias, Ident)?;
            let _ = next!(alias, Punct, as_char => '=')?;
            let mut alias = alias.collect();
            let _ = parsed_aliases.resolve(&mut alias)?;
            if parsed_aliases
                .0
                .insert(alias_name.to_string(), alias.to_string())
                .is_some()
            {
                return Err(Error::new("duplicate alias name in alias file"));
            }
        }
        Ok(parsed_aliases)
    }

    pub(super) fn get() -> Result<&'static Self> {
        static ALIASES: OnceLock<Aliases> = OnceLock::new();

        if ALIASES.get().is_none() {
            let _ = ALIASES.set(Self::parse()?);
        }
        Ok(ALIASES.get().expect("error getting aliases"))
    }

    pub(super) fn create_trigger() -> Result<impl Iterator<Item = TokenTree>> {
        let mut alias_file = env::current_dir()
            .map_err(|x| Error::new_from(x, "getting current directory"))?;
        alias_file.push(Self::FILE);

        let alias_file = alias_file
            .into_os_string()
            .into_string()
            .map_err(|_| Error::new("current directory is not utf-8"))?;

        Ok(tokens!(
            Ident::new("const", Span::call_site()),
            Ident::new("_", Span::call_site()),
            Punct::new(':', Spacing::Alone),
            Punct::new('&', Spacing::Alone),
            Punct::new('\'', Spacing::Joint),
            Ident::new("static", Span::call_site()),
            Group::new(
                Delimiter::Bracket,
                path!("core", "primitive", "u8").collect(),
            ),
            Punct::new('=', Spacing::Alone),
        )
        .chain(super::core_macro("include_bytes", &alias_file)))
    }
}
