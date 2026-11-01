// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! Headless local model-view commands.

pub(super) const COMMANDS: &[&str] = &[
    "model-views",
    "create-model-view",
    "update-model-view",
    "delete-model-view",
    "create-view-folder",
    "delete-view-folder",
];

pub(super) const HELP: &str = concat!(
    "Local model views (presentation only, views.usda; no model revision): ",
    "model-views --store ROOT; create-model-view --store ROOT --name NAME ",
    "[--parallel xy|xz|yz --offset MM | --plane OX,OY,OZ,XX,XY,XZ,YX,YY,YZ | ",
    "--two-points X,Y,Z,X,Y,Z [--work-plane FRAME] | --three-points X,Y,Z,X,Y,Z,X,Y,Z | ",
    "--grid NAME_OR_ID --axis LABEL_OR_ID | --grid NAME_OR_ID --axes all|u|v|first|last | ",
    "--grid NAME_OR_ID --u-axes none|first|last|all --v-axes none|first|last|all ",
    "[--u-prefix TEXT --v-prefix TEXT --u-properties VIEW --v-properties VIEW] | ",
    "--level NAME_OR_ID | --levels all|first|last | ",
    "--element ID [--side front|back|top|bottom|end|3d|four] [--undeformed true] | ",
    "--group ASSEMBLY_OR_PART_ID [--side front|back|top|bottom|end|3d|four] | ",
    "--ray-origin X,Y,Z --ray-direction X,Y,Z [--edge X,Y,Z,X,Y,Z]] ",
    "[--projection orthographic|perspective --span MM --depth none|ABOVE,BELOW ",
    "--rotation Z_DEGREES,X_DEGREES --render-mode MODE --grids true|false ",
    "--hidden-kinds KIND,... --work-area none|MINX,MINY,MINZ,MAXX,MAXY,MAXZ ",
    "--color default|R,G,B --opacity 0..1 --folder PATH --favorite true|false ",
    "--temporary true|false --new-id UUIDV7 --reserved-views 0..256]; update-model-view --store ROOT --id UUIDV7 ",
    "[same properties and single-view construction]; update-model-view --store ROOT --all true ",
    "[--color default|R,G,B --opacity 0..1]; delete-model-view --store ROOT --id UUIDV7; ",
    "create-view-folder --store ROOT --folder PATH; delete-view-folder --store ROOT --folder PATH ",
    "(must contain no views). Up to 256 views, 512 folders and 1 MiB; temporary views are returned ",
    "but omitted from disk. Curved grid axes and undeformed representations without supported ",
    "geometry are refused. Sharing and drawing creation are separate capabilities.",
);
