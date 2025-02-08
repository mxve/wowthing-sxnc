# wowthing-sxnc

Alternative cross-platform [wowthing](https://wowthing.org) sync implementation written in Rust.

> [!NOTE]  
> Requires a [wowthing.org](https://wowthing.org) account and the [wowthing-collector](https://github.com/ThingEngineering/wowthing-collector) addon.

## 🔧 Setup

1. Download the [latest release](https://github.com/mxve/wowthing-sxnc/releases/latest)
1. Rename `wowthing-sxnc.toml.example` to `wowthing-sxnc.toml`
2. Edit the config file with your settings
   - Optionally move the config file to `~/.config/wowthing-sxnc/wowthing-sxnc.toml` or `%appdata%/wowthing-sxnc/wowthing-sxnc.toml`
3. Run the program (on startup)

## 🔨 Building

```bash
cargo build --release
```

## 📝 Credits

[wowthing.org](https://wowthing.org) for providing the platform and the original [wowthing-sync](https://github.com/ThingEngineering/wowthing-sync) implementation.
