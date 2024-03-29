use crate::{digest::Digest, tree::Tree, Result};

use camino::Utf8Path;
use color_eyre::eyre::eyre;

impl super::Repo {
    pub fn show_head(&self, oid: Option<Digest>) -> Result<()> {
        let oid = match oid {
            Some(oid) => oid,
            None => self.read_head()?.ok_or_else(|| {
                eyre!(
                    /*
                    ⠀⣞⢽⢪⢣⢣⢣⢫⡺⡵⣝⡮⣗⢷⢽⢽⢽⣮⡷⡽⣜⣜⢮⢺⣜⢷⢽⢝⡽⣝
                     ⠸⡸⠜⠕⠕⠁⢁⢇⢏⢽⢺⣪⡳⡝⣎⣏⢯⢞⡿⣟⣷⣳⢯⡷⣽⢽⢯⣳⣫⠇
                     ⠀⠀⢀⢀⢄⢬⢪⡪⡎⣆⡈⠚⠜⠕⠇⠗⠝⢕⢯⢫⣞⣯⣿⣻⡽⣏⢗⣗⠏⠀
                     ⠀⠪⡪⡪⣪⢪⢺⢸⢢⢓⢆⢤⢀⠀⠀⠀⠀⠈⢊⢞⡾⣿⡯⣏⢮⠷⠁⠀⠀
                     ⠀⠀⠀⠈⠊⠆⡃⠕⢕⢇⢇⢇⢇⢇⢏⢎⢎⢆⢄⠀⢑⣽⣿⢝⠲⠉⠀⠀⠀⠀
                     ⠀⠀⠀⠀⠀⡿⠂⠠⠀⡇⢇⠕⢈⣀⠀⠁⠡⠣⡣⡫⣂⣿⠯⢪⠰⠂⠀⠀⠀⠀
                     ⠀⠀⠀⠀⡦⡙⡂⢀⢤⢣⠣⡈⣾⡃⠠⠄⠀⡄⢱⣌⣶⢏⢊⠂⠀⠀⠀⠀⠀⠀
                     ⠀⠀⠀⠀⢝⡲⣜⡮⡏⢎⢌⢂⠙⠢⠐⢀⢘⢵⣽⣿⡿⠁⠁⠀⠀⠀⠀⠀⠀⠀
                     ⠀⠀⠀⠀⠨⣺⡺⡕⡕⡱⡑⡆⡕⡅⡕⡜⡼⢽⡻⠏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
                     ⠀⠀⠀⠀⣼⣳⣫⣾⣵⣗⡵⡱⡡⢣⢑⢕⢜⢕⡝⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
                     ⠀⠀⠀⣴⣿⣾⣿⣿⣿⡿⡽⡑⢌⠪⡢⡣⣣⡟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
                     ⠀⠀⠀⡟⡾⣿⢿⢿⢵⣽⣾⣼⣘⢸⢸⣞⡟⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
                     ⠀⠀⠀⠀⠁⠇⠡⠩⡫⢿⣝⡻⡮⣒⢽⠋⠀
                    */
                    "No HEAD"
                )
            })?,
        };

        let commit = self
            .database
            .load(&oid)?
            .into_commit()
            .ok_or_else(|| eyre!("The provided oid does not point to a committ: {:x}", oid))?;
        let tree = self.database.load(commit.tree_id())?.into_tree().unwrap();

        self.show_tree(&tree, "".into())?;

        Ok(())
    }

    fn show_tree(&self, tree: &Tree, prefix: &Utf8Path) -> Result<()> {
        for entry in tree.entries().values() {
            let (oid, name, mode) = match entry {
                crate::tree::TreeEntry::File(file) => (file.oid(), file.name(), file.mode()),
                crate::tree::TreeEntry::IncompleteFile { oid, name, mode } => {
                    (oid, name.as_str(), *mode)
                }
                crate::tree::TreeEntry::Directory { tree, name } => {
                    let prefix = Utf8Path::new(&name);
                    self.show_tree(tree, prefix)?;
                    continue;
                }
            };

            println!("{:o} {:x} {}", mode, oid, prefix.join(name));

            // let object = self.database.load(oid)?;
            // if let Some(tree) = object.as_tree() {
            // unreachable!()
            // } else {
            // }
        }

        Ok(())
    }
}
