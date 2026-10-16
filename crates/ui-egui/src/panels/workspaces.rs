//! Window ▸ Workspaces, like Premiere: the built-in and the user's own workspaces in one
//! alphabetical list, then Reset to Saved Layout, Save Changes to this Workspace, Save as New
//! Workspace… and Edit Workspaces… (rename, delete). Saved layouts live in `workspaces.json` beside the
//! preferences file. A built-in workspace can be changed and reset but not deleted or renamed;
//! deleting its saved changes brings back the original.

use egui::Align2;
use serde_json::{Value, json};

use crate::FilmcraftApp;
use crate::dock::{self, is_builtin, slug};
use crate::menus::MenuItem;
use crate::state::WorkspaceDialog;

/// Command ids under `window.workspace.` that are not workspace names.
const ACTIONS: [&str; 8] = ["reset", "saveChanges", "saveAs", "edit", "rename", "delete", "export", "import"];

/// `window.workspace.*`: switch (`window.workspace.<name>`), `reset`, `saveChanges`,
/// `saveAs {name?}` (no name: open the dialog), `edit`, `rename {from, to}`, `delete {name}`,
/// `export {name?, path?}`, `import {path?}`.
pub fn route(app: &mut FilmcraftApp, id: &str, params: &Value) -> Option<Result<Value, String>> {
    let ws = id.strip_prefix("window.workspace.")?;
    let arg = |k: &str| params.get(k).and_then(Value::as_str).map(str::to_string);
    let r = match ws {
        "reset" => {
            let n = arg("name").unwrap_or_else(|| app.ui.workspace.clone());
            app.workspaces.drafts.retain(|d| d.name != n);
            let _ = app.save_workspaces();
            if app.ui.workspace == n {
                let layout = dock::saved_docking(&app.workspaces, &n);
                app.ui.dock = layout.root;
                app.ui.floating = layout.floating;
                app.ui.keys.maximized = None;
            }
            Ok(json!({"workspace": n, "reset": n}))
        }
        "saveChanges" => {
            let n = arg("name").unwrap_or_else(|| app.ui.workspace.clone());
            save(app, &n).map(|_| json!({"saved": n}))
        }
        "saveAs" => match arg("name") {
            Some(n) => save_new(app, &n),
            None => {
                app.ui.workspace_dialog = Some(WorkspaceDialog::SaveAs { name: tl!("Untitled Workspace").into() });
                Ok(json!({"dialog": "saveWorkspace"}))
            }
        },
        "edit" => {
            app.ui.workspace_dialog = Some(WorkspaceDialog::Edit { selected: None, name: String::new() });
            Ok(json!({"dialog": "editWorkspaces", "workspaces": dock::names(&app.workspaces)}))
        }
        "rename" => rename(app, &arg("from").unwrap_or_default(), &arg("to").unwrap_or_default()),
        "delete" => delete(app, &arg("name").unwrap_or_default()),
        "export" => match arg("path") {
            Some(path) => {
                let name = arg("name").unwrap_or_else(|| app.ui.workspace.clone());
                export(app, &name, &path)
            }
            None => {
                let name = arg("name").unwrap_or_else(|| app.ui.workspace.clone());
                let filter = tl!("FilmCraft Workspace");
                let suggested = format!("{}.fcws", slug(&name));
                let dialog = crate::native_dialogs::FileDialog::save_as(filter, &["fcws", "json"], &suggested);
                app.pick(dialog, move |app, paths| {
                    let Some(path) = paths.into_iter().next() else { return Ok(json!({"cancelled": true})) };
                    let r = export(app, &name, &path);
                    if let Ok(_) = &r {
                        app.ui.status = tlf!("Exported workspace “{name}”", name);
                    }
                    r
                })
            }
        },
        "import" => match arg("path") {
            Some(path) => import(app, &path),
            None => {
                let filter = tl!("FilmCraft Workspace");
                let dialog = crate::native_dialogs::FileDialog::open_file(filter, &["fcws", "json"]);
                app.pick(dialog, |app, paths| {
                    let Some(path) = paths.into_iter().next() else { return Ok(json!({"cancelled": true})) };
                    let r = import(app, &path);
                    if let Ok(v) = &r {
                        let name = v["imported"].as_str().unwrap_or_default();
                        app.ui.status = tlf!("Imported workspace “{name}”", name);
                    }
                    r
                })
            }
        },
        key => match dock::names(&app.workspaces).into_iter().find(|n| slug(n) == key) {
            Some(n) => {
                app.set_workspace(&n);
                Ok(json!({"workspace": n}))
            }
            None => Err(format!("unknown workspace `{key}`")),
        },
    };
    Some(r)
}

/// Save the current layout as the saved layout of `name`.
fn save(app: &mut FilmcraftApp, name: &str) -> Result<(), String> {
    crate::dock::Layout { root: app.ui.dock.clone(), floating: app.ui.floating.clone() }.validate()?;
    let layout = serde_json::to_value(&app.ui.dock).map_err(|e| e.to_string())?;
    let floating = if app.ui.floating.is_empty() { serde_json::Value::Null } else { serde_json::to_value(&app.ui.floating).map_err(|e| e.to_string())? };
    let mut next = app.workspaces.clone();
    let saved = &mut next.saved;
    match saved.iter_mut().find(|s| s.name == name) {
        Some(s) => {
            s.layout = layout;
            s.floating = floating;
        }
        None => saved.push(dock::SavedWorkspace { name: name.to_string(), layout, floating }),
    }
    next.drafts.retain(|d| d.name != name);
    next.current = name.to_string();
    app.set_workspaces(next)
}

/// A trimmed new workspace name, or why it can't be used.
fn new_name(app: &FilmcraftApp, name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("the workspace needs a name".into());
    }
    if ACTIONS.iter().any(|a| a.eq_ignore_ascii_case(&slug(name))) {
        return Err(format!("`{name}` can't be used as a workspace name"));
    }
    if let Some(n) = dock::find(&app.workspaces, name) {
        return Err(format!("a workspace named `{n}` already exists"));
    }
    Ok(name.to_string())
}

fn save_new(app: &mut FilmcraftApp, name: &str) -> Result<Value, String> {
    let name = new_name(app, name)?;
    save(app, &name)?;
    app.ui.workspace = name.clone();
    Ok(json!({"saved": name, "workspace": name}))
}

fn rename(app: &mut FilmcraftApp, from: &str, to: &str) -> Result<Value, String> {
    if is_builtin(from) {
        return Err(format!("`{from}` is a built-in workspace and keeps its name"));
    }
    if !app.workspaces.saved.iter().any(|s| s.name == from) && !app.workspaces.drafts.iter().any(|d| d.name == from) {
        return Err(format!("unknown workspace `{from}`"));
    }
    let to = new_name(app, to)?;
    let mut next = app.workspaces.clone();
    for s in next.saved.iter_mut().filter(|s| s.name == from) {
        s.name = to.clone();
    }
    for d in next.drafts.iter_mut().filter(|d| d.name == from) {
        d.name = to.clone();
    }
    if next.current == from {
        next.current = to.clone();
    }
    app.set_workspaces(next)?;
    if app.ui.workspace == from {
        app.ui.workspace = to.clone();
    }
    Ok(json!({"renamed": from, "to": to}))
}

/// Delete a user workspace; on a built-in, drop its saved changes (back to the original).
fn delete(app: &mut FilmcraftApp, name: &str) -> Result<Value, String> {
    let mut next = app.workspaces.clone();
    let before_saved = next.saved.len();
    let before_drafts = next.drafts.len();
    next.saved.retain(|s| s.name != name);
    next.drafts.retain(|d| d.name != name);
    if next.saved.len() == before_saved && next.drafts.len() == before_drafts {
        return Err(if is_builtin(name) { format!("`{name}` is a built-in workspace and can't be deleted") } else { format!("unknown workspace `{name}`") });
    }
    app.set_workspaces(next)?;
    if app.ui.workspace == name {
        app.set_workspace(if is_builtin(name) { name } else { "Editing" });
    }
    Ok(if is_builtin(name) { json!({"restored": name}) } else { json!({"deleted": name}) })
}

/// Export a workspace layout to a file at `path`.
pub fn export(app: &FilmcraftApp, name: &str, path: &str) -> Result<Value, String> {
    let layout = if name == app.ui.workspace {
        crate::dock::Layout { root: app.ui.dock.clone(), floating: app.ui.floating.clone() }
    } else {
        dock::active_docking(&app.workspaces, name)
    };
    layout.validate()?;
    let root_val = serde_json::to_value(&layout.root).map_err(|e| e.to_string())?;
    let floating_val = if layout.floating.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::to_value(&layout.floating).map_err(|e| e.to_string())?
    };
    let payload = json!({
        "filmcraftWorkspace": 1,
        "name": name,
        "layout": root_val,
        "floating": floating_val,
    });
    let bytes = serde_json::to_vec_pretty(&payload).map_err(|e| e.to_string())?;
    let path_buf = std::path::PathBuf::from(path);
    if let Some(p) = path_buf.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(&path_buf, bytes).map_err(|e| format!("saving workspace: {e}"))?;
    Ok(json!({"exported": name, "path": path}))
}

/// Import a workspace layout from a file at `path`.
pub fn import(app: &mut FilmcraftApp, path: &str) -> Result<Value, String> {
    use std::io::Read as _;
    const MAX_BYTES: u64 = 4 * 1024 * 1024;
    let file = std::fs::File::open(path).map_err(|e| format!("cannot open workspace file: {e}"))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes).map_err(|e| format!("reading workspace file: {e}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("workspace file is too large (max 4 MB)".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|e| format!("invalid JSON in workspace file: {e}"))?;
    let layout_val = value.get("layout").cloned().ok_or_else(|| "missing `layout` in workspace file".to_string())?;
    let floating_val = value.get("floating").cloned().unwrap_or(Value::Null);

    let root: dock::DockNode = serde_json::from_value(layout_val.clone()).map_err(|e| format!("invalid dock layout: {e}"))?;
    let floating: Vec<dock::Floating> = if floating_val.is_null() {
        vec![]
    } else {
        serde_json::from_value(floating_val.clone()).map_err(|e| format!("invalid floating panels: {e}"))?
    };
    let layout = dock::Layout { root, floating };
    layout.validate()?;

    let raw_name = value.get("name").and_then(Value::as_str).unwrap_or("").trim();
    let name_candidate = if raw_name.is_empty() {
        std::path::Path::new(path).file_stem().and_then(|s| s.to_str()).unwrap_or("Imported Workspace").trim()
    } else {
        raw_name
    };
    let target_name = if is_builtin(name_candidate) {
        let mut candidate = format!("{name_candidate} (Custom)");
        let mut count: usize = 2;
        while dock::find(&app.workspaces, &candidate).is_some() {
            candidate = format!("{name_candidate} (Custom {count})");
            count = count.saturating_add(1);
        }
        candidate
    } else {
        name_candidate.to_string()
    };

    let mut next = app.workspaces.clone();
    match next.saved.iter_mut().find(|s| s.name == target_name) {
        Some(s) => {
            s.layout = layout_val;
            s.floating = floating_val;
        }
        None => next.saved.push(dock::SavedWorkspace {
            name: target_name.clone(),
            layout: layout_val,
            floating: floating_val,
        }),
    }
    next.drafts.retain(|d| d.name != target_name);
    next.current = target_name.clone();
    app.set_workspaces(next)?;
    app.set_workspace(&target_name);
    Ok(json!({"imported": target_name, "workspace": target_name}))
}

/// Rebuild the Window ▸ Workspaces submenu: every workspace (the current one checked), then the
/// layout commands in table order.
pub fn menu(app: &FilmcraftApp, items: &mut Vec<MenuItem>) {
    splice(items, &app.workspaces, &app.session, Some(&app.ui.workspace), |n| app.ui.language.tr(n).to_string());
}

/// The same submenu without a running app: the built-in workspaces in English, none checked
/// (`menus::default_menu_items`).
pub fn default_menu(items: &mut Vec<MenuItem>, session: &filmcraft_engine::Session) {
    splice(items, &dock::WorkspacePrefs::default(), session, None, str::to_string);
}

fn splice(items: &mut Vec<MenuItem>, prefs: &dock::WorkspacePrefs, session: &filmcraft_engine::Session, current: Option<&str>, tr: impl Fn(&str) -> String) {
    let is_ws = |it: &MenuItem| it.path.len() == 2 && it.path[0] == "Window" && it.path[1] == "Workspaces";
    let Some(at) = items.iter().position(is_ws) else { return };
    let actions: Vec<MenuItem> =
        items.iter().filter(|it| is_ws(it) && it.id.strip_prefix("window.workspace.").is_some_and(|k| ACTIONS.contains(&k))).cloned().collect();
    items.retain(|it| !is_ws(it));
    let mut sub: Vec<MenuItem> = dock::names(prefs)
        .into_iter()
        .map(|n| {
            let id = format!("window.workspace.{}", slug(&n));
            MenuItem {
                label: tr(&n),
                path: vec!["Window".into(), "Workspaces".into()],
                shortcut: session.shortcuts.primary(&id),
                enabled: true,
                checked: current.map(|c| n == c),
                id,
            }
        })
        .collect();
    sub.extend(actions);
    items.splice(at..at, sub);
}

/// Save as New Workspace… and Edit Workspaces… dialogs.
pub fn dialogs(app: &mut FilmcraftApp, ctx: &egui::Context) {
    let Some(mut d) = app.ui.workspace_dialog.clone() else { return };
    let mut elems: Vec<(String, egui::Rect, String)> = Vec::new();
    let mut push = |id: &str, r: &egui::Response, label: &str| elems.push((id.to_string(), r.rect, label.to_string()));
    let mut close = crate::widgets::escape_closes(ctx);
    let mut act: Option<(&str, Value)> = None;
    let names = dock::names(&app.workspaces);
    let changed: Vec<String> = app.workspaces.saved.iter().map(|s| s.name.clone()).collect();
    let (title, shown) = match &d {
        WorkspaceDialog::SaveAs { .. } => ("New Workspace", tl!("New Workspace")),
        WorkspaceDialog::Edit { .. } => ("Edit Workspaces", tl!("Edit Workspaces")),
    };
    egui::Window::new(shown)
        .id(egui::Id::new(title))
        .collapsible(false)
        .resizable(false)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(crate::widgets::dialog_center(ctx))
        .show(ctx, |ui| match &mut d {
            WorkspaceDialog::SaveAs { name } => {
                ui.horizontal(|ui| {
                    ui.label(tl!("Name:"));
                    let r = ui.text_edit_singleline(name);
                    push("workspaces.save.name", &r, "Name");
                });
                ui.horizontal(|ui| {
                    let r = ui.button(tl!("Cancel"));
                    push("workspaces.save.cancel", &r, "Cancel");
                    close |= r.clicked();
                    let r = ui.button(tl!("OK"));
                    push("workspaces.save.ok", &r, "OK");
                    if r.clicked() {
                        act = Some(("window.workspace.saveAs", json!({"name": name.clone()})));
                    }
                });
            }
            WorkspaceDialog::Edit { selected, name } => {
                for (i, n) in names.iter().enumerate() {
                    let r = ui.selectable_label(selected.as_ref() == Some(n), crate::i18n::t(n));
                    push(&format!("workspaces.edit.row.{i}"), &r, n);
                    if r.clicked() {
                        *selected = Some(n.clone());
                        *name = n.clone();
                    }
                }
                ui.separator();
                let sel = selected.clone().filter(|s| names.contains(s));
                let own = sel.as_deref().is_some_and(|s| !is_builtin(s));
                ui.horizontal(|ui| {
                    ui.label(tl!("Name:"));
                    let r = ui.add_enabled(own, egui::TextEdit::singleline(name));
                    push("workspaces.edit.name", &r, "Name");
                    let r = ui.add_enabled(own && sel.as_deref() != Some(name.trim()), egui::Button::new(tl!("Rename")));
                    push("workspaces.edit.rename", &r, "Rename");
                    if r.clicked()
                        && let Some(s) = &sel
                    {
                        act = Some(("window.workspace.rename", json!({"from": s, "to": name.clone()})));
                        *selected = Some(name.trim().to_string());
                    }
                });
                ui.horizontal(|ui| {
                    let r = ui.add_enabled(sel.is_some(), egui::Button::new(tl!("Save Changes to this Workspace")));
                    push("workspaces.edit.saveChanges", &r, "Save Changes to this Workspace");
                    if r.clicked() && let Some(s) = &sel {
                        act = Some(("window.workspace.saveChanges", json!({"name": s})));
                    }
                    let has_mod = sel.as_deref().is_some_and(|s| dock::has_draft(&app.workspaces, s));
                    let r = ui.add_enabled(has_mod, egui::Button::new(tl!("Reset to Saved Layout")));
                    push("workspaces.edit.reset", &r, "Reset to Saved Layout");
                    if r.clicked() && let Some(s) = &sel {
                        act = Some(("window.workspace.reset", json!({"name": s})));
                    }
                });
                ui.horizontal(|ui| {
                    let r = ui.add_enabled(sel.is_some(), egui::Button::new(tl!("Export…")));
                    push("workspaces.edit.export", &r, "Export");
                    if r.clicked() && let Some(s) = &sel {
                        let ws_name = s.clone();
                        let filter = tl!("FilmCraft Workspace");
                        let suggested = format!("{}.fcws", slug(&ws_name));
                        app.pick_ui(crate::native_dialogs::FileDialog::save_as(filter, &["fcws", "json"], &suggested), move |app, paths| {
                            let Some(path) = paths.into_iter().next() else { return };
                            if let Err(e) = export(app, &ws_name, &path) {
                                app.ui.status = e;
                            } else {
                                app.ui.status = tlf!("Exported workspace “{name}”", name = ws_name);
                            }
                        });
                    }
                    let r = ui.button(tl!("Import…"));
                    push("workspaces.edit.import", &r, "Import");
                    if r.clicked() {
                        let filter = tl!("FilmCraft Workspace");
                        app.pick_ui(crate::native_dialogs::FileDialog::open_file(filter, &["fcws", "json"]), |app, paths| {
                            let Some(path) = paths.into_iter().next() else { return };
                            match import(app, &path) {
                                Ok(v) => {
                                    let name = v["imported"].as_str().unwrap_or_default();
                                    app.ui.status = tlf!("Imported workspace “{name}”", name);
                                }
                                Err(e) => app.ui.status = e,
                            }
                        });
                    }
                    // a built-in workspace can't be deleted; its saved changes can
                    let restore = sel.as_deref().is_some_and(|s| is_builtin(s) && changed.iter().any(|c| c == s));
                    let label = if restore { "Restore Original" } else { "Delete" };
                    let r = ui.add_enabled(own || restore, egui::Button::new(if restore { tl!("Restore Original") } else { tl!("Delete") }));
                    push("workspaces.edit.delete", &r, label);
                    if r.clicked()
                        && let Some(s) = &sel
                    {
                        act = Some(("window.workspace.delete", json!({"name": s})));
                        if own {
                            *selected = None;
                            name.clear();
                        }
                    }
                    let r = ui.button(tl!("Close"));
                    push("workspaces.edit.close", &r, "Close");
                    close |= r.clicked();
                });
            }
        });
    for (id, r, l) in elems {
        app.auto.add(&id, r, &l);
    }
    let keep_open = matches!(d, WorkspaceDialog::Edit { .. });
    app.ui.workspace_dialog = Some(d);
    if let Some((id, p)) = act {
        match route(app, id, &p) {
            Some(Err(e)) => app.ui.status = e,
            _ => close |= !keep_open,
        }
    }
    if close {
        app.ui.workspace_dialog = None;
    }
}
