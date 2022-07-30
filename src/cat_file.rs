use std::process::exit;

use crate::interface::CatFile;
use crate::repo::Repo;
use crate::Result;

pub fn handle(repo: &mut Repo, args: &CatFile) -> Result<()> {
    #[allow(unused_variables)]
    match args {
        CatFile::Exists { object } => {
            if repo.database.exists(object) {
                // Validate
                todo!("Validate that object is valid");
            } else {
                eprintln!("Object does not exist: {}", object.to_hex());
                exit(1);
            }
        }
        CatFile::PrettyPrint { object } => {
            todo!()
        }
        CatFile::Type {
            object,
            allow_unknown_type,
        } => {
            todo!()
        }
        CatFile::Size {
            object,
            allow_unknown_type,
        } => {
            todo!()
        }
    }
}
