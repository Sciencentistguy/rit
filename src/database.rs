use crate::storable::Storable;
use crate::util;
use colour_eyre::eyre::ContextCompat;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::Result;
use hex::ToHex;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    pub fn store(&self, obj: &impl Storable) -> Result<()> {
        self.write_object(obj)
    }

    fn write_object(&self, obj: &impl Storable) -> Result<()> {
        let content = obj.format();
        let oid = obj.get_oid();
        let oid_string: String = oid.encode_hex();
        let object_path = {
            let mut p = self.path.to_owned();
            p.push(&oid_string[0..2]);
            p.push(&oid_string[2..]);
            p
        };

        let dirname = object_path.parent().wrap_err("object had no parent")?;

        let temp_path = dirname.join(util::tmp_file_name());

        if !dirname.is_dir() {
            std::fs::create_dir_all(dirname)?;
        }

        let mut file = std::fs::File::create(&temp_path)?;

        let mut e = ZlibEncoder::new(Vec::with_capacity(content.len()), Compression::default());
        e.write_all(content)?;
        let compressed_bytes = e.finish()?;

        file.write_all(&compressed_bytes)?;

        drop(file);

        std::fs::rename(temp_path, object_path)?;

        Ok(())
    }
}
