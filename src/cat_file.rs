use std::process::exit;

use tracing::warn;

use crate::interface::CatFile;
use crate::repo::database::LoadedItem;
use crate::repo::Repo;
use crate::Result;

pub fn handle(repo: &mut Repo, args: &CatFile) -> Result<()> {
    #[allow(unused_variables)]
    match args {
        CatFile::Exists { object } => {
            if repo.database.exists(object) {
                match repo.database.load(object) {
                    Ok(_) => {
                        // Object exists and is valid
                        Ok(())
                    }
                    Err(_) => {
                        // Object is corrupt
                        eprintln!("Object is corrupt: {}", object.to_hex());
                        exit(1);
                    }
                }
            } else {
                eprintln!("Object does not exist: {}", object.to_hex());
                exit(1);
            }
        }
        CatFile::PrettyPrint { object_ref } => {
            let object = {
                let loaded = repo.read_ref(object_ref)?;
                match loaded {
                    None => {
                        eprintln!("Invalid object ref: {}", object_ref);
                        exit(1);
                    }
                    Some(oid) => {
                        if !repo.database.exists(&oid) {
                            eprintln!("Object does not exist: {:x}", oid);
                            exit(1);
                        } else {
                            oid
                        }
                    }
                }
            };

            let object = repo.database.load(&object)?;

            match object {
                LoadedItem::Blob(blob) => blob.pretty_print()?,
                LoadedItem::Commit(commit) => commit.pretty_print()?,
                LoadedItem::Tree(tree) => tree.pretty_print()?,
            };

            Ok(())
        }
        CatFile::Type {
            object,
            allow_unknown_type,
        } => {
            if *allow_unknown_type {
                warn!("--allow-unknown-type is not implemented");
            }
            println!("{}", repo.database.load(object)?.kind());
            Ok(())
        }

        CatFile::Size {
            object,
            allow_unknown_type,
        } => {
            todo!()
        }
    }
}
