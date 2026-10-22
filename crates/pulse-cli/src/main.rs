//! `cargo pulse`: check and render multiparty protocol files.
//!
//! Protocol files use the text format described in `README.md`. Install with
//! `cargo install --path crates/pulse-cli`, then run `cargo pulse check <file>`.

use std::{env, fs, process::ExitCode};

use pulse_multiparty::{Action, GlobalProtocol, Program};

const USAGE: &str = "usage: cargo pulse <command> <protocol-file>

commands:
  check    verify the protocol and print each role's projected type
  mermaid  print the protocol as a Mermaid sequence diagram";

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    // `cargo pulse ...` runs this binary with `pulse` as the first argument.
    if args.first().map(String::as_str) == Some("pulse") {
        args.remove(0);
    }
    match args.as_slice() {
        [command, path] if command == "check" => run(path, check),
        [command, path] if command == "mermaid" => run(path, mermaid),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Read and parse the file, then print the command's output or its error.
fn run(path: &str, command: fn(&GlobalProtocol) -> Result<String, String>) -> ExitCode {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::from(2);
        }
    };
    let protocol = match GlobalProtocol::parse(&text) {
        Ok(protocol) => protocol,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match command(&protocol) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{path}: {error}");
            ExitCode::FAILURE
        }
    }
}

fn check(protocol: &GlobalProtocol) -> Result<String, String> {
    let session = protocol.project().map_err(|error| error.to_string())?;
    let mut output = String::from("protocol is well formed and deadlock-free\n");
    for program in session.programs() {
        match projected_type(program) {
            Some(ty) => output.push_str(&format!("{}: {ty}\n", program.role)),
            None => {
                output.push_str(&format!("{}:\n", program.role));
                outline(&program.actions, 1, &mut output);
            }
        }
    }
    Ok(output)
}

fn mermaid(protocol: &GlobalProtocol) -> Result<String, String> {
    Ok(protocol.to_mermaid())
}

/// The `pulse-core` type for one role, for example `Send<Req, Recv<Ok, Close>>`.
///
/// Returns `None` for roles with choices or loops. Those have no single
/// `pulse-core` type, so they are shown as an outline instead.
fn projected_type(program: &Program) -> Option<String> {
    if program.actions.iter().any(Action::is_branching) {
        return None;
    }
    let ty =
        program
            .actions
            .iter()
            .rev()
            .fold(String::from("Close"), |rest, action| match action {
                Action::Send { label, .. } => format!("Send<{label}, {rest}>"),
                Action::Recv { label, .. } => format!("Recv<{label}, {rest}>"),
                _ => rest,
            });
    Some(ty)
}

/// An indented outline of a role's actions, for roles with choices or loops.
fn outline(actions: &[Action], depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    for action in actions {
        match action {
            Action::Send { to, label } => out.push_str(&format!("{indent}send {label} to {to}\n")),
            Action::Recv { from, label } => {
                out.push_str(&format!("{indent}receive {label} from {from}\n"));
            }
            Action::Choose { to, branches } => {
                out.push_str(&format!(
                    "{indent}choose a branch and announce it to {}\n",
                    to.join(", ")
                ));
                outline_branches(branches, depth, out);
            }
            Action::Offer { from, branches } => {
                out.push_str(&format!("{indent}wait for the branch chosen by {from}\n"));
                outline_branches(branches, depth, out);
            }
            Action::Loop(body) => {
                out.push_str(&format!("{indent}loop:\n"));
                outline(body, depth + 1, out);
            }
            Action::Continue => out.push_str(&format!("{indent}continue the loop\n")),
        }
    }
}

fn outline_branches(branches: &[Vec<Action>], depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth + 1);
    for (index, branch) in branches.iter().enumerate() {
        out.push_str(&format!("{indent}branch {index}:\n"));
        outline(branch, depth + 2, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_prints_each_roles_projected_type() {
        let protocol =
            GlobalProtocol::parse("Client -> Server : Req; Server -> Client : Ok;").expect("valid");
        let output = check(&protocol).expect("well formed");
        assert!(
            output.contains("Client: Send<Req, Recv<Ok, Close>>"),
            "{output}"
        );
        assert!(
            output.contains("Server: Recv<Req, Send<Ok, Close>>"),
            "{output}"
        );
    }

    #[test]
    fn check_reports_a_malformed_protocol() {
        let protocol = GlobalProtocol::parse("A -> A : Ping;").expect("parses");
        assert_eq!(
            check(&protocol),
            Err("role `A` sends `Ping` to itself".into())
        );
    }

    #[test]
    fn mermaid_matches_the_library_output() {
        let protocol = GlobalProtocol::parse("A -> B : Hi;").expect("valid");
        assert_eq!(mermaid(&protocol), Ok(protocol.to_mermaid()));
    }
}
