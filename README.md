# tilecap

Screenshot tool for tiling WMs. X11.

```
tilecap -m full              # full screen → file
tilecap -m window            # click a window → file
tilecap -m region            # drag to select → file
tilecap -m region -o clipboard
tilecap -r 800x600+100+50
```

## Install

```
cargo build --release
install -m755 target/release/tilecap ~/.local/bin/
```

Packages: `make deb`, `make rpm`. PKGBUILD and Gentoo ebuild included.

Dep: `xclip` for clipboard support.

## Usage

```
-m full|window|region    mode
-o file|clipboard        output target
-d <dir>                 output directory
-n <name>                filename stem
-r <WxH+X+Y>             non-interactive geometry
-c <path>                config path
```

Config: `~/.config/tilecap/config.toml`
```toml
output_dir = "/home/user/Pictures/Screenshots"
```

MIT.
