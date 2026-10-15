// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Apibim

//! The boundary rules, applied to an in-memory workspace and module table.
//!
//! Nothing here touches the disk, so every failing path is tested with synthetic input and no
//! broken manifest is ever committed.

use std::fmt;

use crate::table::{Kind, ModuleTable, Status};
use crate::workspace::Workspace;

/// APIs that a domain crate must not use outside test code (document section 1, rule 2).
pub const FORBIDDEN_APIS: [&str; 6] = [
    "std::fs",
    "std::process",
    "std::net",
    "std::thread",
    "std::env",
    "std::os",
];

/// One broken rule. The text of [`fmt::Display`] names the rule and the offending item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// Two table rows share a package name.
    DuplicateName(String),
    /// Two table rows share a path.
    DuplicatePath(String),
    /// A row allows a package that has no row.
    UnknownAllowed {
        /// Row package.
        package: String,
        /// The unknown dependency.
        dependency: String,
    },
    /// A row allows a dependency that is not in a strictly lower layer.
    NotLowerLayer {
        /// Row package.
        package: String,
        /// The offending dependency.
        dependency: String,
    },
    /// A workspace member has no row.
    MemberNotInTable {
        /// Member package.
        package: String,
        /// Member path.
        path: String,
    },
    /// A member's path differs from its row.
    PathMismatch {
        /// Member package.
        package: String,
        /// Path in the table.
        table: String,
        /// Path in the workspace.
        actual: String,
    },
    /// A member's row says `planned`.
    MemberNotNow(String),
    /// A `now` row has no workspace member.
    RowNotMember(String),
    /// An internal dependency that the row does not list.
    EdgeNotAllowed {
        /// Depending package.
        from: String,
        /// Dependency.
        to: String,
        /// Manifest table that declares it.
        section: String,
    },
    /// A domain crate depends on a host, app or tool.
    DomainDependsOnNative {
        /// Domain package.
        from: String,
        /// The non-domain dependency.
        to: String,
    },
    /// Some package depends on an app or a tool.
    DependsOnAppOrTool {
        /// Depending package.
        from: String,
        /// The app or tool.
        to: String,
    },
    /// A domain crate uses a third-party crate that is not on the allowlist.
    ForbiddenThirdParty {
        /// Domain package.
        package: String,
        /// The third-party dependency.
        dependency: String,
        /// Manifest table that declares it.
        section: String,
    },
    /// A tool is publishable.
    ToolPublishable(String),
    /// Non-test source of a domain crate mentions a forbidden API.
    ForbiddenApi {
        /// Domain package.
        package: String,
        /// Source path.
        file: String,
        /// The mentioned API path.
        api: &'static str,
    },
    /// The CI wasm check names crates by hand.
    CiHardCodedList(String),
    /// The CI file never asks the tool for the portable list.
    CiMissingPortableCall,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateName(p) => write!(f, "table: package `{p}` has more than one row"),
            Self::DuplicatePath(p) => write!(f, "table: path `{p}` has more than one row"),
            Self::UnknownAllowed {
                package,
                dependency,
            } => {
                write!(
                    f,
                    "table: `{package}` allows `{dependency}`, which has no row"
                )
            }
            Self::NotLowerLayer {
                package,
                dependency,
            } => write!(
                f,
                "table: `{package}` allows `{dependency}`, which is not in a strictly lower layer"
            ),
            Self::MemberNotInTable { package, path } => {
                write!(
                    f,
                    "workspace member `{package}` ({path}) is missing from the module table"
                )
            }
            Self::PathMismatch {
                package,
                table,
                actual,
            } => write!(
                f,
                "`{package}` is at `{actual}` but the module table says `{table}`"
            ),
            Self::MemberNotNow(p) => {
                write!(
                    f,
                    "`{p}` is a workspace member but its table status is `planned`"
                )
            }
            Self::RowNotMember(p) => {
                write!(
                    f,
                    "table row `{p}` is `now` but it is not a workspace member"
                )
            }
            Self::EdgeNotAllowed { from, to, section } => write!(
                f,
                "`{from}` depends on `{to}` in [{section}], which its table row does not allow"
            ),
            Self::DomainDependsOnNative { from, to } => {
                write!(
                    f,
                    "domain crate `{from}` depends on non-domain crate `{to}`"
                )
            }
            Self::DependsOnAppOrTool { from, to } => {
                write!(f, "`{from}` depends on app or tool `{to}`")
            }
            Self::ForbiddenThirdParty {
                package,
                dependency,
                section,
            } => write!(
                f,
                "domain crate `{package}` depends on `{dependency}` in [{section}], which is not on the third-party allowlist"
            ),
            Self::ToolPublishable(p) => write!(f, "tool `{p}` must set publish = false"),
            Self::ForbiddenApi { package, file, api } => {
                write!(
                    f,
                    "domain crate `{package}` uses `{api}` in `{file}` outside test code"
                )
            }
            Self::CiHardCodedList(line) => write!(
                f,
                "CI wasm check lists crates by hand (`{}`); derive them with `beamcraft-boundaries portable`",
                line.trim()
            ),
            Self::CiMissingPortableCall => f.write_str(
                "CI never runs `beamcraft-boundaries -- portable` to derive the wasm crate list",
            ),
        }
    }
}

/// Checks the table itself: unique names and paths, known dependencies, strictly lower layers and
/// the kind rules.
pub fn check_table(table: &ModuleTable) -> Vec<Violation> {
    let mut out = Vec::new();
    for (i, row) in table.rows.iter().enumerate() {
        let later = || table.rows.iter().skip(i.saturating_add(1));
        if later().any(|r| r.package == row.package) {
            out.push(Violation::DuplicateName(row.package.clone()));
        }
        if later().any(|r| r.path == row.path) {
            out.push(Violation::DuplicatePath(row.path.clone()));
        }
        for dep in &row.allowed {
            let Some(target) = table.row(dep) else {
                out.push(Violation::UnknownAllowed {
                    package: row.package.clone(),
                    dependency: dep.clone(),
                });
                continue;
            };
            if target.layer >= row.layer {
                out.push(Violation::NotLowerLayer {
                    package: row.package.clone(),
                    dependency: dep.clone(),
                });
            }
            out.extend(kind_rules(
                &row.package,
                row.kind,
                &target.package,
                target.kind,
            ));
        }
    }
    out
}

fn kind_rules(from: &str, from_kind: Kind, to: &str, to_kind: Kind) -> Vec<Violation> {
    let mut out = Vec::new();
    if from_kind == Kind::Domain && to_kind != Kind::Domain {
        out.push(Violation::DomainDependsOnNative {
            from: from.to_owned(),
            to: to.to_owned(),
        });
    }
    if matches!(to_kind, Kind::App | Kind::Tool) {
        out.push(Violation::DependsOnAppOrTool {
            from: from.to_owned(),
            to: to.to_owned(),
        });
    }
    out
}

/// The first forbidden API each line of non-test source mentions.
///
/// Everything from the first `#[cfg(test)]` line on is test code and is not scanned; whole-line
/// `//` comments are skipped.
pub fn forbidden_apis(source: &str) -> Vec<&'static str> {
    let mut found = Vec::new();
    for line in source.lines().map(str::trim_start) {
        if line.starts_with("#[cfg(test)]") {
            break;
        }
        if line.starts_with("//") {
            continue;
        }
        for api in FORBIDDEN_APIS {
            let mentioned = line.match_indices(api).any(|(at, _)| {
                let after = line.get(at.saturating_add(api.len())..).unwrap_or("");
                !after.starts_with(|c: char| c.is_alphanumeric() || c == '_')
            });
            if mentioned && !found.contains(&api) {
                found.push(api);
            }
        }
    }
    found
}

/// Checks a workspace against the module table.
pub fn check_workspace(workspace: &Workspace, table: &ModuleTable) -> Vec<Violation> {
    let mut out = check_table(table);
    for package in &workspace.packages {
        let Some(row) = table.row(&package.name) else {
            out.push(Violation::MemberNotInTable {
                package: package.name.clone(),
                path: package.path.clone(),
            });
            continue;
        };
        if row.path != package.path {
            out.push(Violation::PathMismatch {
                package: package.name.clone(),
                table: row.path.clone(),
                actual: package.path.clone(),
            });
        }
        if row.status != Status::Now {
            out.push(Violation::MemberNotNow(package.name.clone()));
        }
        if row.kind == Kind::Tool && !package.publish_false {
            out.push(Violation::ToolPublishable(package.name.clone()));
        }
        for dep in &package.dependencies {
            match table.row(&dep.name) {
                Some(target) if target.package != package.name => {
                    if !row.allowed.contains(&dep.name) {
                        out.push(Violation::EdgeNotAllowed {
                            from: package.name.clone(),
                            to: dep.name.clone(),
                            section: dep.section.clone(),
                        });
                    }
                    out.extend(kind_rules(
                        &package.name,
                        row.kind,
                        &target.package,
                        target.kind,
                    ));
                }
                Some(_) => out.push(Violation::EdgeNotAllowed {
                    from: package.name.clone(),
                    to: dep.name.clone(),
                    section: dep.section.clone(),
                }),
                None if row.kind == Kind::Domain && !table.third_party.contains(&dep.name) => {
                    out.push(Violation::ForbiddenThirdParty {
                        package: package.name.clone(),
                        dependency: dep.name.clone(),
                        section: dep.section.clone(),
                    });
                }
                None => {}
            }
        }
        if row.kind == Kind::Domain {
            for source in &package.sources {
                for api in forbidden_apis(&source.text) {
                    out.push(Violation::ForbiddenApi {
                        package: package.name.clone(),
                        file: source.path.clone(),
                        api,
                    });
                }
            }
        }
    }
    for row in &table.rows {
        if row.status == Status::Now && !workspace.packages.iter().any(|p| p.name == row.package) {
            out.push(Violation::RowNotMember(row.package.clone()));
        }
    }
    out
}

/// Checks that the CI file derives its wasm crate list instead of repeating it.
pub fn check_ci(ci: &str) -> Vec<Violation> {
    let mut out = Vec::new();
    for line in ci.lines() {
        if line.contains("wasm32-unknown-unknown") && line.contains("-p beamcraft-") {
            out.push(Violation::CiHardCodedList(line.to_owned()));
        }
    }
    if !ci.contains("beamcraft-boundaries -- portable") {
        out.push(Violation::CiMissingPortableCall);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Dependency;
    use crate::table::Row;
    use crate::workspace::{Package, Source};

    fn row(
        package: &str,
        path: &str,
        kind: Kind,
        layer: u8,
        allowed: &[&str],
        status: Status,
    ) -> Row {
        Row {
            package: package.into(),
            path: path.into(),
            kind,
            layer,
            allowed: allowed.iter().map(|s| (*s).into()).collect(),
            status,
            owner: "T".into(),
        }
    }

    fn table() -> ModuleTable {
        ModuleTable {
            rows: vec![
                row("geom", "crates/geom", Kind::Domain, 0, &[], Status::Now),
                row(
                    "model",
                    "crates/model",
                    Kind::Domain,
                    1,
                    &["geom"],
                    Status::Now,
                ),
                row(
                    "host",
                    "crates/host",
                    Kind::Host,
                    4,
                    &["model"],
                    Status::Planned,
                ),
                row(
                    "cli",
                    "apps/cli",
                    Kind::App,
                    5,
                    &["model", "host"],
                    Status::Now,
                ),
                row("fix", "tools/fix", Kind::Tool, 9, &[], Status::Now),
            ],
            third_party: vec!["allowed-lib".into()],
        }
    }

    fn pkg(name: &str, path: &str, deps: &[&str]) -> Package {
        Package {
            name: name.into(),
            path: path.into(),
            publish_false: name == "fix",
            dependencies: deps
                .iter()
                .map(|d| Dependency {
                    name: (*d).into(),
                    section: "dependencies".into(),
                })
                .collect(),
            sources: Vec::new(),
        }
    }

    fn good() -> Workspace {
        Workspace {
            packages: vec![
                pkg("geom", "crates/geom", &[]),
                pkg("model", "crates/model", &["geom"]),
                pkg("cli", "apps/cli", &["model"]),
                pkg("fix", "tools/fix", &[]),
            ],
        }
    }

    fn with(mut ws: Workspace, edit: impl FnOnce(&mut Workspace)) -> Vec<Violation> {
        edit(&mut ws);
        check_workspace(&ws, &table())
    }

    fn pkg_mut<'a>(ws: &'a mut Workspace, name: &str) -> &'a mut Package {
        ws.packages.iter_mut().find(|p| p.name == name).unwrap()
    }

    #[test]
    fn a_consistent_workspace_passes() {
        assert_eq!(check_workspace(&good(), &table()), []);
    }

    #[test]
    fn rejects_a_back_edge() {
        let v = with(good(), |ws| {
            pkg_mut(ws, "geom").dependencies.push(Dependency {
                name: "model".into(),
                section: "dependencies".into(),
            })
        });
        assert!(v.contains(&Violation::EdgeNotAllowed {
            from: "geom".into(),
            to: "model".into(),
            section: "dependencies".into()
        }));
    }

    #[test]
    fn rejects_an_edge_in_a_dev_dependency_table_too() {
        let v = with(good(), |ws| {
            pkg_mut(ws, "model").dependencies.push(Dependency {
                name: "cli".into(),
                section: "dev-dependencies".into(),
            })
        });
        assert!(v.iter().any(|x| matches!(x, Violation::EdgeNotAllowed { section, .. } if section == "dev-dependencies")));
        assert!(v.contains(&Violation::DependsOnAppOrTool {
            from: "model".into(),
            to: "cli".into()
        }));
        assert!(v.contains(&Violation::DomainDependsOnNative {
            from: "model".into(),
            to: "cli".into()
        }));
    }

    #[test]
    fn rejects_a_domain_crate_depending_on_the_host() {
        let v = with(good(), |ws| {
            pkg_mut(ws, "model").dependencies.push(Dependency {
                name: "host".into(),
                section: "dependencies".into(),
            })
        });
        assert!(v.contains(&Violation::DomainDependsOnNative {
            from: "model".into(),
            to: "host".into()
        }));
    }

    #[test]
    fn rejects_an_unlisted_forward_edge() {
        let v = with(good(), |ws| {
            pkg_mut(ws, "cli").dependencies.push(Dependency {
                name: "geom".into(),
                section: "dependencies".into(),
            })
        });
        assert!(v.contains(&Violation::EdgeNotAllowed {
            from: "cli".into(),
            to: "geom".into(),
            section: "dependencies".into()
        }));
    }

    #[test]
    fn rejects_a_self_dependency() {
        let v = with(good(), |ws| {
            pkg_mut(ws, "model").dependencies.push(Dependency {
                name: "model".into(),
                section: "dependencies".into(),
            })
        });
        assert!(v.iter().any(|x| matches!(x, Violation::EdgeNotAllowed { from, to, .. } if from == "model" && to == "model")));
    }

    #[test]
    fn domain_crates_may_use_only_allowlisted_third_party_crates() {
        let v = with(good(), |ws| {
            let m = pkg_mut(ws, "model");
            for name in ["tokio", "allowed-lib"] {
                m.dependencies.push(Dependency {
                    name: name.into(),
                    section: "dependencies".into(),
                });
            }
        });
        assert_eq!(
            v,
            [Violation::ForbiddenThirdParty {
                package: "model".into(),
                dependency: "tokio".into(),
                section: "dependencies".into()
            }]
        );
    }

    #[test]
    fn apps_may_use_third_party_crates() {
        let v = with(good(), |ws| {
            pkg_mut(ws, "cli").dependencies.push(Dependency {
                name: "clap".into(),
                section: "dependencies".into(),
            })
        });
        assert_eq!(v, []);
    }

    #[test]
    fn rejects_a_member_missing_from_the_table() {
        let v = with(good(), |ws| {
            ws.packages.push(pkg("extra", "crates/extra", &[]))
        });
        assert_eq!(
            v,
            [Violation::MemberNotInTable {
                package: "extra".into(),
                path: "crates/extra".into()
            }]
        );
    }

    #[test]
    fn rejects_a_now_row_without_a_member() {
        let v = with(good(), |ws| ws.packages.retain(|p| p.name != "fix"));
        assert_eq!(v, [Violation::RowNotMember("fix".into())]);
    }

    #[test]
    fn rejects_a_planned_row_that_already_exists_and_a_moved_path() {
        let v = with(good(), |ws| {
            ws.packages.push(pkg("host", "crates/elsewhere", &[]))
        });
        assert!(v.contains(&Violation::MemberNotNow("host".into())));
        assert!(v.contains(&Violation::PathMismatch {
            package: "host".into(),
            table: "crates/host".into(),
            actual: "crates/elsewhere".into()
        }));
    }

    #[test]
    fn rejects_a_publishable_tool() {
        let v = with(good(), |ws| pkg_mut(ws, "fix").publish_false = false);
        assert_eq!(v, [Violation::ToolPublishable("fix".into())]);
    }

    #[test]
    fn rejects_forbidden_apis_in_domain_sources_only_outside_tests() {
        let v = with(good(), |ws| {
            pkg_mut(ws, "model").sources.push(Source {
                path: "crates/model/src/lib.rs".into(),
                text: "// std::fs in a comment is fine\nuse std::fs::File;\nfn f() { let _ = std::env::args(); }\n#[cfg(test)]\nmod t { use std::process::Command; }\n".into(),
            });
            pkg_mut(ws, "cli").sources.push(Source {
                path: "apps/cli/src/main.rs".into(),
                text: "use std::fs;\n".into(),
            });
        });
        let apis: Vec<&str> = v
            .iter()
            .filter_map(|x| match x {
                Violation::ForbiddenApi { api, .. } => Some(*api),
                _ => None,
            })
            .collect();
        assert_eq!(apis, ["std::fs", "std::env"]);
    }

    #[test]
    fn api_names_must_match_whole_paths() {
        assert_eq!(
            forbidden_apis("use std::fsx::Y;\nuse std::environment;\n"),
            Vec::<&str>::new()
        );
        assert_eq!(forbidden_apis("use std::{fs, io};\n"), Vec::<&str>::new());
        assert_eq!(forbidden_apis("use std::net::TcpStream;\n"), ["std::net"]);
        assert_eq!(
            forbidden_apis("let t = std::thread::spawn(f);\n"),
            ["std::thread"]
        );
    }

    #[test]
    fn table_rules_catch_bad_layers_unknown_names_and_duplicates() {
        let mut t = table();
        t.rows.push(row(
            "geom",
            "crates/other",
            Kind::Domain,
            0,
            &["ghost"],
            Status::Planned,
        ));
        t.rows.push(row(
            "loop",
            "crates/loop",
            Kind::Domain,
            1,
            &["model", "geom", "loop"],
            Status::Planned,
        ));
        t.rows.push(row(
            "up",
            "crates/up",
            Kind::Domain,
            1,
            &["cli", "fix"],
            Status::Planned,
        ));
        let v = check_table(&t);
        assert!(v.contains(&Violation::DuplicateName("geom".into())));
        assert!(v.contains(&Violation::UnknownAllowed {
            package: "geom".into(),
            dependency: "ghost".into()
        }));
        assert!(v.contains(&Violation::NotLowerLayer {
            package: "loop".into(),
            dependency: "model".into()
        }));
        assert!(v.contains(&Violation::NotLowerLayer {
            package: "loop".into(),
            dependency: "loop".into()
        }));
        assert!(v.contains(&Violation::NotLowerLayer {
            package: "up".into(),
            dependency: "cli".into()
        }));
        assert!(v.contains(&Violation::DomainDependsOnNative {
            from: "up".into(),
            to: "cli".into()
        }));
        assert!(v.contains(&Violation::DependsOnAppOrTool {
            from: "up".into(),
            to: "fix".into()
        }));
        let dup_path = {
            let mut t = table();
            t.rows.push(row(
                "z",
                "crates/geom",
                Kind::Domain,
                0,
                &[],
                Status::Planned,
            ));
            check_table(&t)
        };
        assert_eq!(dup_path, [Violation::DuplicatePath("crates/geom".into())]);
        assert_eq!(check_table(&table()), []);
    }

    #[test]
    fn ci_must_derive_the_wasm_list() {
        let ok = "run: |\n  list=\"$(cargo run -p beamcraft-boundaries -- portable)\"\n  cargo check --target wasm32-unknown-unknown $list\n";
        assert_eq!(check_ci(ok), []);
        let by_hand = "run: cargo check --target wasm32-unknown-unknown -p beamcraft-geom\n";
        let v = check_ci(by_hand);
        assert_eq!(v.len(), 2);
        assert!(matches!(v.first(), Some(Violation::CiHardCodedList(_))));
        assert!(v.contains(&Violation::CiMissingPortableCall));
    }

    #[test]
    fn every_violation_has_a_readable_message() {
        let v = with(good(), |ws| {
            pkg_mut(ws, "model").dependencies.push(Dependency {
                name: "tokio".into(),
                section: "dependencies".into(),
            });
        });
        let text = v
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("`model`") && text.contains("`tokio`"),
            "{text}"
        );
    }
}
