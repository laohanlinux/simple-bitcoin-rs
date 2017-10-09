use std::io;
use std::path::{Path, PathBuf};

use quick_error::ResultExt;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        NoFileName{}
        Io(err: io::Error, path: PathBuf){
            display("could not read file {:?}: {}", path, err)
            context(path: &'a Path, err: io::Error)
                -> (err, path.to_path_buf())
        }
    }
}
