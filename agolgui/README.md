# AGOL GUI

A native desktop version of AgolTui built with Rust and egui. It runs on Linux and Windows and keeps network/reference analysis off the UI thread.

## Features

- Browse and search all organization content by title, owner, or item ID
- Inspect item metadata and incoming references
- Explore an interactive connection graph from feature layers to web maps to web applications
- Filter source items with zero references
- Summarize item counts by owner
- Review broken data connections
- Review items whose data does not match the expected AGOL structure
- Open items directly in ArcGIS Online
- Retry authentication or loading failures without restarting the application

## Connection graph controls

- Drag an empty area to pan around the graph
- Use the mouse wheel or trackpad to zoom
- Drag individual nodes to reposition them
- Select a node to inspect it and open it in ArcGIS Online
- Inspect named incoming and outgoing connections and select connected nodes directly
- Pause, resume, reset, or fit the force-directed layout from the toolbar

Blue nodes are feature layers/services, green nodes are web maps, and orange nodes are web applications.

## Requirements

Set the same OAuth environment variables used by AgolTui:

```text
ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_ID
ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_SECRET
```

The OAuth application needs General and Admin View privileges for members, groups, and content.

## Run on Linux

Install the native development packages required by winit/Wayland/X11 for your distribution, then run:

```bash
cargo run --release
```

## Build for Windows

From a Windows Rust toolchain:

```powershell
cargo build --release
```

The executable is created at `target\release\agolgui.exe`.

## Test

```bash
cargo test
```

This project currently references the neighboring `agol` crate through `../../agol`. Update that path in `Cargo.toml` if the projects are stored elsewhere.
