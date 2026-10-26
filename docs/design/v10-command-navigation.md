# V10 command navigation

An experimental UI shell: command categories and a workspace drawer. This is not completion of
the V10 acceptance gate or an implementation of the commands shown as placeholders.

The model header has a vertical-ellipsis button that opens a left-aligned modal drawer, with
groups for model files, exchange/output, project preferences, support and the session. All its
command entries are disabled, including file and session entries. They dispatch no commands.
The close button, Esc and a click on the backdrop dismiss the drawer. Its overlay keeps the
viewport's dimensions and camera unchanged, and blocks interaction with the model behind it.

Five ribbon tabs organize commands: Model, Modify, View, Drawings and Project. Tab navigation is
active. Existing create/edit tools keep their CLI paths; the additional modelling, transformation,
view, drawing and project buttons are disabled and explain their availability on hover. Tabs and
tools scroll horizontally when the window is narrow. Navigation changes no model data.

The layout, labels and grouping are contributor-original BeamCraft code. Appearance uses the
existing BeamCraft theme, Inter font and procedural icons. No new asset or dependency is added;
no third-party screenshot, icon, palette or other asset is included or traced.

Verification uses the real egui input/layout path: switching to Modify, opening the drawer,
clicking an inert file command and dismissing with Esc preserve the snapshot and start no edit.
The drawer test checks that its command widgets are disabled. Existing UI-to-CLI end-to-end tests
continue to cover the implemented tools.
