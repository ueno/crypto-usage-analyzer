# Crypto Usage Analyzer - Developer Documentation

## Interaction Details

### Hover Interaction
Move your mouse over segments to highlight them and see detailed tooltips showing:
- Operation name with parameters (e.g., "tls::verify [rsa_pss_rsae_sha256]")
- Total count of operations
- Number of child operations
- Top 5 child operations with percentages

### Click to Zoom
- Click on any segment to zoom into that subtree (a banner will appear at the top)
- Click on the center/root segment or use the "Reset" button in the banner to return to the full view

## Data Format

The application expects JSON data in the crypto-auditing event format:

```json
[
  {
    "context": "unique-context-id",
    "origin": "origin-hash",
    "start": 12345678,
    "end": 12345678,
    "events": {
      "name": "tls::handshake_client",
      "tls::protocol_version": 772
    },
    "spans": [...]
  }
]
```

## Architecture

### Core Components

- **data.rs**: Data structures for parsing audit events and building tree representation
- **sunburst_chart/**: Custom GTK4 widget for rendering the interactive sunburst chart using Cairo
  - **mod.rs**: Public API and GObject wrapper subclassing gtk4::DrawingArea
  - **imp.rs**: Internal implementation with drawing logic and event handling
- **main.rs**: Libadwaita application setup with modern GNOME patterns:
  - AdwApplicationWindow for the main window
  - AdwHeaderBar with hamburger menu
  - AdwNavigationSplitView for two-pane layout
  - AdwNavigationPage for sidebar and content areas
  - GtkTreeView with GtkTreeStore for hierarchical event display
  - AdwToolbarView for proper header integration
  - AdwStatusPage for empty state
  - AdwBanner for zoom notifications with reset button
  - AdwAboutWindow for application information
  - GMenu-based primary menu with actions
  - Stack-based navigation between empty and content states

## Color Scheme

Colors are generated based on a hash of the operation name and depth, ensuring consistent colors for the same operations while providing visual distinction between different types of cryptographic operations.

## UI Layout

The application uses a two-pane layout:
- **Left Sidebar**: Hierarchical tree view of all events with operation names and counts
- **Right Content Area**: Interactive sunburst chart visualization
  - **Center**: Root node containing all events
  - **First ring**: Context groups
  - **Outer rings**: Individual cryptographic operations expanding outward with increasing detail

The sidebar can be resized by dragging the divider between the panes.
