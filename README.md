# Attr Alias

This crate allows defining arbitrary aliases for attributes.

[![GitHub Build Status](https://github.com/dylni/attr_alias/workflows/build/badge.svg?branch=master)](https://github.com/dylni/attr_alias/actions?query=branch%3Amaster)

## Usage

Add the following lines to your "Cargo.toml" file:

```toml
[dependencies]
attr_alias = "0.1"
```

See the [documentation] for available functionality and examples.

## Rust version support

The minimum supported Rust toolchain version is currently Rust 1.70.0.

Minor version updates may increase this version requirement. However, the
previous two Rust releases will always be supported. If the minimum Rust
version must not be increased, use a tilde requirement to prevent updating this
crate's minor version:

```toml
[dependencies]
attr_alias = "~0.1"
```

## License

Licensing terms are specified in [COPYRIGHT].

Unless you explicitly state otherwise, any contribution submitted for inclusion
in this crate, as defined in [LICENSE-APACHE], shall be licensed according to
[COPYRIGHT], without any additional terms or conditions.

[COPYRIGHT]: https://github.com/dylni/attr_alias/blob/master/COPYRIGHT
[documentation]: https://docs.rs/attr_alias
[LICENSE-APACHE]: https://github.com/dylni/attr_alias/blob/master/LICENSE-APACHE
