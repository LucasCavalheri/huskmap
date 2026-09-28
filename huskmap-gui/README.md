# huskmap-gui

Freya desktop map. Talks only to `huskmap-core`.

- `view_model.rs`: every screen rule as plain data (nodes, legend, alarms, ledger, drawer, confirm, keyboard). Tested without Skia.
- `theme.rs`: Afterlife tokens. Colors, type scale, spacing and motion timings live here and nowhere else.
- `icons.rs`: Hugeicons glyphs and the real brand marks, one table. Sources in `assets/icons/LICENSE.md` and `assets/brands/SOURCES.md`.
- `fonts.rs`: Instrument Serif (display) and IBM Plex Mono (paths, bytes), embedded, SIL OFL.
- `query.rs`: the filter language behind the search box and the chips. Tested without Skia.
- `components/`: rail, dial, list and filter chips, drawer, confirm, guide, chrome, empty states.

## The dial

Angle is kind (six fixed sectors), radius is age (log of hours: the last hour hugs the core, a year is the rim), size is weight, color is what you are allowed to do: verdigris free, amber caution, copper guarded, oxblood untouchable. Occupied husks pulse. The sonar beam lights husks as it passes, and spins faster while a scan streams husks in.

## Frames

```
cargo test -p huskmap-gui --features desktop --test snapshots
# PNGs in target/tmp/snapshots/
HUSKMAP_SNAPSHOT_REPORT=~/.local/state/huskmap/last-report.json cargo test -p huskmap-gui --features desktop --test snapshots
```

Every screen at every window size (1120×720 up to 2560×1440), and motion clips, all offscreen:

```
scripts/screenshots.sh                  # NAME.png + NAME@WxH.png per size
scripts/screenshots.sh --sizes 1366x768 # one size
scripts/screenshots.sh --video          # + clips/NAME.mp4 and clips/NAME-strip.png (12 frames, needs ffmpeg)
```

Never commit frames rendered from a real home.
