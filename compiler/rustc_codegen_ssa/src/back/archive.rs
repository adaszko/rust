use rustc_data_structures::temp_dir::MaybeTempDir;
use rustc_session::cstore::DllImport;
use rustc_session::Session;
use rustc_span::symbol::Symbol;

use std::io;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(super) fn find_library(
    name: Symbol,
    verbatim: bool,
    search_paths: &[PathBuf],
    sess: &Session,
) -> PathBuf {
    // On Windows, static libraries sometimes show up as libfoo.a and other
    // times show up as foo.lib
    let oslibname = if verbatim {
        name.to_string()
    } else {
        format!("{}{}{}", sess.target.staticlib_prefix, name, sess.target.staticlib_suffix)
    };
    let unixlibname = format!("lib{}.a", name);

    for path in search_paths {
        debug!("looking for {} inside {:?}", name, path);
        let test = path.join(&oslibname);
        if test.exists() {
            return test;
        }
        if oslibname != unixlibname {
            let test = path.join(&unixlibname);
            if test.exists() {
                return test;
            }
            if cfg!(target_os = "macos") {
                // This is a bit unintuitive: On macOS, a shared library may not be present on the file
                // system yet still be loadable.  This is due to the fact that there's a global cache of shared libraries being
                // maintained by the system on BigSur and newer [1].  Therefore, as a last resort, try loading the library
                // instead of just checking for its existence as a file.
                // [1]: https://web.archive.org/web/20220317152936/https://developer.apple.com/documentation/macos-release-notes/macos-big-sur-11-beta-release-notes
                let path: &OsStr = test.as_ref();
                if let Ok(_) = unsafe { libloading::Library::new(path) } {
                    return test;
                }
            }
        }
    }
    sess.fatal(&format!(
        "could not find native static library `{}`, \
                         perhaps an -L flag is missing?",
        name
    ));
}

pub trait ArchiveBuilder<'a> {
    fn new(sess: &'a Session, output: &Path, input: Option<&Path>) -> Self;

    fn add_file(&mut self, path: &Path);
    fn remove_file(&mut self, name: &str);
    fn src_files(&mut self) -> Vec<String>;

    fn add_archive<F>(&mut self, archive: &Path, skip: F) -> io::Result<()>
    where
        F: FnMut(&str) -> bool + 'static;

    fn build(self);

    fn inject_dll_import_lib(
        &mut self,
        lib_name: &str,
        dll_imports: &[DllImport],
        tmpdir: &MaybeTempDir,
    );
}
