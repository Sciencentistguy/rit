use crate::Result;

impl super::Repo {
    pub fn branch(&mut self, name: Option<&str>, delete: bool) -> Result<()> {
        if let Some(name) = name {
            self.create_branch(name)?;
        } else {
            eprintln!("NYI");
        }
        Ok(())
    }
}
