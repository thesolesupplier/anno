use git2::Repository;

fn main() {
    let url = "https://github.com/constantincerdan/photography-website";
    let repo = match Repository::clone(url, "./test_repo") {
        Ok(repo) => repo,
        Err(e) => panic!("failed to clone: {}", e),
    };
}
