use std::{ffi::OsStr, path::Path};

use cairo_lang_compiler::project::ProjectError;
use cairo_lang_defs::ids::ModuleId;
use cairo_lang_filesystem::{
    db::{CrateConfiguration, FilesGroupEx},
    ids::{CrateId, Directory},
};
use cairo_lang_semantic::db::SemanticGroup;

/// Set up the 'db' to compile the file at the given path.
/// Returns the id of the generated crate.
pub fn setup_single_file_project(
    db: &mut dyn SemanticGroup,
    path: &Path,
) -> Result<CrateId, ProjectError> {
    match path.extension().and_then(OsStr::to_str) {
        Some("cairo") => (),
        _ => {
            return Err(ProjectError::BadFileExtension);
        }
    }
    if !path.exists() {
        return Err(ProjectError::NoSuchFile {
            path: path.to_string_lossy().to_string(),
        });
    }
    let bad_path_err = || ProjectError::BadPath {
        path: path.to_string_lossy().to_string(),
    };
    let canonical = path.canonicalize().map_err(|_| bad_path_err())?;
    let file_dir = canonical.parent().ok_or_else(bad_path_err)?;
    let file_stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(bad_path_err)?;
    if file_stem == "lib" {
        let crate_name = file_dir.to_str().ok_or_else(bad_path_err)?;
        let crate_id = CrateId::plain(db, crate_name);
        db.set_crate_config(
            crate_id,
            Some(CrateConfiguration::default_for_root(Directory::Real(
                file_dir.to_path_buf(),
            ))),
        );
        Ok(crate_id)
    } else {
        // If file_stem is not lib, create a fake lib file.
        let crate_id = CrateId::plain(db, file_stem);
        db.set_crate_config(
            crate_id,
            Some(CrateConfiguration::default_for_root(Directory::Real(
                file_dir.to_path_buf(),
            ))),
        );

        let module_id = ModuleId::CrateRoot(crate_id);
        let file_id = db.module_main_file(module_id).unwrap();
        db.override_file_content(file_id, Some(format!("mod {file_stem};").into()));
        Ok(crate_id)
    }
}
