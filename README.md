# wowthing-sxnc

Alternative cross-platform [wowthing](https://wowthing.org) sync implementation written in Rust.

> [!IMPORTANT]
> Requires a [wowthing.org](https://wowthing.org) account/api key and the [wowthing-collector](https://github.com/ThingEngineering/wowthing-collector) addon.

## Setup

> [!NOTE]
> wowthing-sxnc can be configured with the config file or by using command line arguments. When running with CLI args the `--api-key` and `--watch-folder` arguments are required

> [!NOTE]
> wowthing-sxnc does not automatically register to run on startup, do it yourself using a daemon, your desktop environments autostart feature, windows task scheduler or shell:startup.


### Usage

```bash
# Run with config file / Open interactive config builder
wowthing-sxnc

# Run with command line arguments
wowthing-sxnc --api-key <key> --watch-folder "/path/to/wow/_retail_"

# Run once and exit
wowthing-sxnc --once

# Override config values
wowthing-sxnc --watch-interval 30 --upload-host "https://example.com/"
```

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `--api-key` | API key from wowthing.org | |
| `--watch-folder` | Path to WoW _retail_ folder | |
| `--upload-host` | Upload host | `https://wowthing.org/` |
| `--upload-on-startup` | Upload files on startup | `false` |
| `--watch-interval` | Watch interval in seconds | 60 |
| `--once` | Run once and exit | `false` |

## Building

```bash
cargo build --release
```

## Credits

[wowthing.org](https://wowthing.org) for providing the platform and the original [wowthing-sync](https://github.com/ThingEngineering/wowthing-sync) implementation.