use once_cell::sync::OnceCell;

impl super::Repo {
    pub fn ignores(&self) -> &[String] {
        static IGNORES: OnceCell<Vec<String>> = OnceCell::new();

        IGNORES.get_or_init(|| {
            let global_ignore = [".git".to_owned()];
            let gitignore =
                std::fs::read_to_string(self.dir.join(".gitignore")).unwrap_or_default();
            let repo_ignore = gitignore.lines().map(|x| x.to_string());

            global_ignore.into_iter().chain(repo_ignore).collect()
        })
    }
}
