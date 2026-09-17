# AgolTui and AGOL GUI user guide

AgolTui provides two interfaces for exploring ArcGIS Online organization content:

- **AgolTui** is a keyboard-driven terminal interface.
- **AGOL GUI** is a mouse-driven desktop interface with an interactive connection graph.

Both applications authenticate with the same ArcGIS Online OAuth application and perform the
same organization-wide analysis. Use the TUI for quick keyboard-driven inspection or the GUI
when you want visual navigation and a graph of item relationships.

## Before you start

You need:

- A Rust toolchain with `cargo` available.
- A local checkout of the companion `agol` crate. The TUI expects it at `../agol`, relative to
  this repository; the GUI expects the same checkout through `../../agol` from `agolgui/`.
- Either an ArcGIS Online user account that can view organization content and members, or an
  ArcGIS Online OAuth 2.0 application with **General** and **Admin View** privileges for members,
  groups, and content.

For unattended authentication, set the OAuth application credentials in the shell from which
you will launch either application.

Linux/macOS:

```bash
export ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_ID="your-client-id"
export ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_SECRET="your-client-secret"
```

Windows PowerShell:

```powershell
$env:ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_ID = "your-client-id"
$env:ORG_WIDE_SEARCH_AND_CATALOG_CLIENT_SECRET = "your-client-secret"
```

Do not commit the client secret to this repository. If your shell saves environment assignments
in its history, use your operating system's secret-management mechanism instead. The organization
ID and URL are discovered automatically during authentication.

If either variable is missing or empty, the apps fall back to an interactive ArcGIS Online user
login. The TUI prompts in the terminal and hides password input. The GUI displays a masked login
form. Credentials are sent only to ArcGIS Online over HTTPS, are not logged or saved, and the
returned token is verified against the current-user endpoint before organization data is loaded.
Accounts that use a federated identity provider or multifactor authentication may require OAuth
application credentials instead of the direct user login.

## Choose an interface

| Capability | TUI | GUI |
| --- | --- | --- |
| Browse organization content | Yes | Yes |
| Search by title, owner, or item ID | Yes | Yes |
| Inspect incoming references | Yes | Yes |
| Inspect both incoming and outgoing connections | Limited | Yes |
| Find items with zero references | Yes | Yes |
| Summarize items by owner | Yes | Yes |
| Review broken connections and structure issues | Yes | Yes |
| Open an item in ArcGIS Online | Displays URL | Opens browser |
| Interactive relationship graph | No | Yes |

## Use the terminal interface

### Launch the TUI

From the repository root, run:

```bash
cargo run --release
```

The application first authenticates and downloads the organization's content. It then analyzes
references and loads organization users in the background. The **References** panel displays
`Loading references...` until reference analysis is ready.

You can optionally apply an initial filter from the command line:

```bash
# Match an owner or username (case-insensitive substring)
cargo run --release -- --email alice

# Match an item title (case-insensitive substring)
cargo run --release -- --search roads

# Require both the owner and title filters
cargo run --release -- --email alice --search roads

# Match an ArcGIS Online item ID (substring)
cargo run --release -- --item-id abc123
```

If `--item-id` is supplied with another filter, the item-ID filter takes precedence.

### Understand the main screen

The screen has three sections:

1. **AGOL Content List** shows matching item IDs and the current item count.
2. **Layer Info** shows the selected item's title, type, owner, and active filters.
3. **References** shows items that reference the selected item, including each item's title,
   type, and ArcGIS Online URL.

Press `Tab` to move focus between the content list and references table. The focused section has
a dark background. `j`/Down and `k`/Up move within whichever section is focused; selection wraps
at the first and last row.

### Search and filter

Press `s` or `i` to open search. Starting a new search restores the full content list and clears
previous filters.

While the search screen is open:

| Key | Action |
| --- | --- |
| `F1` | Search item titles by keyword |
| `F2` | Search owners/usernames |
| `F3` | Search item IDs |
| Any printable character | Add text at the cursor |
| Backspace | Delete the character before the cursor |
| Enter | Apply the selected search |
| Esc | Cancel the search and reset all filters |

The search screen previews matching values as you type. Title and item-ID searches must contain
between 3 and 50 characters. Owner searches use a case-insensitive substring match; item-ID
matching is case-sensitive.

Press `0` on the main screen to keep source items that have no incoming references. Service
Definition items are excluded from this result. This filter is applied to the currently visible
list, so it can be combined with an initial command-line filter. Press `Esc` to restore all
content and clear active filters.

### Inspect organization health

- Press `u` to replace the main screen with a table of item totals by owner. Press `Esc` to
  return to the full content list.
- Press uppercase `B` (`Shift+B`) to list items that refer to a source item that is no longer
  present in the organization. Use `j`/Down and `k`/Up to move through the table, and `Esc` to
  return.
- Press uppercase `M` (`Shift+M`) to list items whose data could not be analyzed using the
  expected ArcGIS Online structure. The table includes the item ID, title, type, owner, and
  reason. Use `j`/Down and `k`/Up to move, and `Esc` to return.

Reference analysis runs in the background. Wait for the loading message to disappear before
interpreting the zero-reference or broken-connection results.

### TUI key reference

| Key | Context | Action |
| --- | --- | --- |
| `j` or Down | Lists and tables | Move down |
| `k` or Up | Lists and tables | Move up |
| `Tab` | Main screen | Switch between content and references |
| `s` or `i` | Main screen | Open search |
| `0` | Main screen | Show items with zero references |
| `u` | Main screen | Show item totals by owner |
| `Shift+B` | Main screen | Show broken connections |
| `Shift+M` | Main screen | Show unexpected item structures |
| `Esc` | Any screen | Cancel, return, and/or reset filters |
| `q` | Normal mode | Quit |

## Use the desktop interface

### Launch the GUI

From the repository root, run:

```bash
cd agolgui
cargo run --release
```

On Linux, the GUI requires the native development packages used by `winit` for Wayland or X11.
The exact package names depend on your distribution. On Windows, run the same command from a
Rust-enabled PowerShell terminal. To build without launching:

```powershell
cargo build --release
```

The Windows executable is written to `agolgui\target\release\agolgui.exe` when the command is
run inside the `agolgui` directory.

At startup, a status screen reports authentication, content loading, and reference analysis.
These requests run in the background, and the navigation counts populate when loading finishes.
If loading fails, check the displayed message and credentials, then select **Retry**.

### Content view

The **Content** tab is the default view.

1. Choose **Title**, **Owner**, or **Item ID** from the filter menu.
2. Type in the filter box. Results update immediately using a case-insensitive substring match.
3. Optionally enable **Zero references only**. Service Definition items are excluded.
4. Select an item in the left pane to inspect it in the right pane.

The details pane displays the item's title, type, owner, access level, item ID, optional snippet,
and connections:

- **Uses / incoming** lists organization items referenced by the selected item.
- **Used by / outgoing** lists organization items that reference the selected item.

Select **Open in ArcGIS Online** to open the item, or select a connected item's title to open
that item. **Clear** empties the text filter and disables the zero-reference filter.

### Connections view

The **Connections** tab visualizes supported relationships from feature layers and services to
web maps and web applications. Node colors identify the item category:

- Blue: feature layers and services
- Green: web maps
- Orange: web applications and related app types

Use the graph as follows:

- Drag empty space to pan.
- Use the mouse wheel or trackpad to zoom.
- Drag a node to reposition and pin it.
- Select a node to open its details panel.
- Select an item in the **Incoming** or **Outgoing** list to move directly to that node.
- Select **Pause layout** or **Resume layout** to control the force-directed simulation.
- Select **Reset layout** to generate a fresh layout and resume simulation.
- Select **Fit graph** to center and scale all nodes into the available area.
- Select **Close details** to hide the node panel.

In the graph details panel, **Incoming** means items the selected node uses, while **Outgoing**
means items that use the selected node. **Open in ArcGIS Online** launches the selected item in
your default browser.

### Other GUI views

- **Owners** shows an alphabetical table of owners and their item counts. It also reports how
  many members were returned by the organization directory.
- **Broken connections** lists dependent items that refer to a source item no longer present in
  the organization. Select an item title to open it in ArcGIS Online.
- **Structure issues** lists items that could not be analyzed using the expected schema, along
  with the reported reason. Hover over a long reason to read it, and select a title to open the
  item.

The numbers shown in the navigation tabs are result counts, not loading progress.

## Interpreting the results

The applications describe references from the perspective of a selected source item:

```text
feature layer  <- used by -  web map  <- used by -  application
```

An item with **zero references** has no known organization item depending on it. That can make it
a cleanup candidate, but it is not proof that the item is unused: external applications,
hard-coded URLs, unsupported item types, and data outside the organization may not appear in the
analysis.

A **broken connection** identifies an item that refers to a source ID absent from the downloaded
organization content. Confirm permissions and item ownership before concluding that the source
was deleted; content the OAuth application cannot view may also appear absent.

A **structure issue** means the analyzer could not interpret an item's data using its expected
schema. It does not necessarily mean the ArcGIS Online item itself is invalid.

## Troubleshooting

### Authentication fails

- If using application authentication, confirm both environment variables are set in the same
  terminal, check them for typing errors, and verify the application's privileges.
- If using user login, confirm the username and password and verify that the account belongs to
  an ArcGIS Online organization and can view its content.
- Federated or multifactor-authenticated accounts may need the OAuth application flow.
- Restart the TUI after correcting credentials. In the GUI, select **Retry** to return to the
  login form when environment credentials are absent.

### Loading takes a long time

Both interfaces download all visible organization content and inspect item data. Large
organizations can take time to analyze. The GUI shows the current loading phase; the TUI lets you
browse content while references and users continue loading.

### Results are empty or incomplete

- Clear filters (`Esc` in the TUI or **Clear** in the GUI).
- Wait for reference analysis to finish before using reference-based views.
- Verify that the OAuth application can view the expected content and members.
- Remember that the connection graph intentionally includes only supported feature-layer,
  web-map, and application categories.

### The GUI does not open a browser

The GUI delegates links to the operating system's default browser. Copy the displayed item ID
and open it manually at:

```text
https://YOUR-ORGANIZATION/home/item.html?id=ITEM_ID
```
