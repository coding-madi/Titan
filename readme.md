
# Table of Contents

1.  [Log monitoring](#orgc191dd1)
    1.  [Dependencies](#orgcdeb91a)
        1.  [Linker](#org8ad1c6d)
        2.  [development tools](#org4ea4c9f)


<a id="orgc191dd1"></a>

# Log monitoring


<a id="orgcdeb91a"></a>

## Dependencies


<a id="org8ad1c6d"></a>

### Linker

The linking takes most of the time during the development cycle. A faster linker would go a long way.

    sudo yum install lld clang

Add the below linkers to the `.cargo/config.toml` file

    [target.x86_64-unknown-linux-gnu]
    rustflags = ["-C", "linker=clang", "-C", "link-arg=-fuse-ld=lld"]


<a id="org4ea4c9f"></a>

### development tools

Install cargo watch

    cargo install cargo-watch

    rustup component add rustfmt

    rustup component add clippy

