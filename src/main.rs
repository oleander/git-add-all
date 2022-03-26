use git2::Repository;
use std::env;
use std::option::Option;
use std::path::Path;

pub struct State {
    branch: String,
    args: Vec<String>,
}

impl State {
    fn message(&self) -> String {
        self.split_message().join(" ")
    }

    fn split_message(&self) -> Vec<String> {
        if self.args_has_ticket_id() {
            return self.args.clone();
        }

        if let Some(id) = self.branch_ticket_id() {
            let mut args = self.args.clone();
            args.insert(0, id);
            return args.clone();
        }

        self.args.clone()
    }

    fn args_has_ticket_id(&self) -> bool {
        if let Some(arg) = self.args.to_vec().first() {
            arg.starts_with("KDB-")
        } else {
            false
        }
    }

    fn branch_ticket_id(&self) -> Option<String> {
        if !self.branch.starts_with("KDB-") {
            return None;
        }

        if let Some(index) = self.branch.find("/") {
            return Some(self.branch[..index].to_string());
        }

        Some(self.branch.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_master_and_without_ticket_id() {
        let state = State {
            branch: "master".to_string(),
            args: vec!["foo".to_string(), "bar".to_string()],
        };
        assert!(!state.args_has_ticket_id());
        assert_eq!(state.message(), "foo bar".to_string());
    }

    #[test]
    fn with_kdb_and_without_ticket_id() {
        let state = State {
            branch: "KDB-123".to_string(),
            args: vec!["foo".to_string(), "bar".to_string()],
        };

        assert!(!state.args_has_ticket_id());
        assert_eq!(state.message(), "KDB-123 foo bar".to_string());
    }

    #[test]
    fn with_kdb_and_with_ticket_id() {
        let state = State {
            branch: "KDB-123".to_string(),
            args: vec!["KDB-456".to_string(), "bar".to_string()],
        };

        assert!(state.args_has_ticket_id());
        assert_eq!(state.message(), "KDB-456 bar".to_string());
    }

    #[test]
    fn without_kdb_with_ticket_id() {
        let state = State {
            branch: "master".to_string(),
            args: vec!["KDB-456".to_string(), "foo".to_string(), "bar".to_string()],
        };

        assert!(state.args_has_ticket_id());
        assert_eq!(state.message(), "KDB-456 foo bar".to_string());
    }

    #[test]
    fn without_kdb_without_ticket_id() {
        let state = State {
            branch: "master".to_string(),
            args: vec!["foo".to_string(), "bar".to_string()],
        };

        assert!(!state.args_has_ticket_id());
        assert_eq!(state.message(), "foo bar".to_string());
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let bin = env::args().next().expect("No binary name");

    if args.is_empty() {
        eprintln!("Usage: {} [ticket-id] (optional) [message]", bin);
        std::process::exit(1);
    }

    let curren_path = env::current_dir().expect("Failed to get current path");
    let repo = Repository::discover(curren_path).expect("No repository found");
    let mut index = repo.index().expect("Index not found");
    let mut is_empty = true;

    let head = repo.head().expect("No HEAD found");
    let branch = head.shorthand().expect("No branch found");

    let state = State {
        branch: branch.to_string(),
        args: args,
    };

    let message = state.message();

    for result in repo.statuses(None).unwrap().iter() {
        let path = Path::new(result.path().expect("No path found"));
        match result.status() {
            git2::Status::CURRENT | git2::Status::INDEX_NEW | git2::Status::INDEX_MODIFIED => {
                is_empty = false;
                index.add_path(path).expect("Failed to add path");
            }
            git2::Status::WT_NEW | git2::Status::WT_MODIFIED => {
                is_empty = false;
                index.add_path(path).expect("Failed to add path");
            }
            git2::Status::WT_DELETED => {
                is_empty = false;
                index.remove_path(path).expect("Failed to remove path");
            }
            _ => {}
        }
    }

    if is_empty {
        eprintln!("Nothing to commit");
        std::process::exit(1);
    }

    index.write().expect("Could not write index");

    let tree_id = index.write_tree().expect("Could not write tree");
    let tree = repo.find_tree(tree_id).expect("Could not find tree");
    let user = &repo.signature().expect("Could not get signature");
    let head_id = repo.head().expect("Could not get head id");
    let head = head_id.target().expect("Could not get head");
    let parents = &[&repo.find_commit(head).expect("Could not find commit")];

    let oid = repo
        .commit(Some("HEAD"), user, user, &message, &tree, parents)
        .expect("Could not commit");

    let commit = repo
        .find_commit(oid)
        .expect("Could not newly created commit");
    println!("{:?}", commit);
}
