use std::env;

use nix::unistd::Uid;

pub fn get_run_user_dir() -> String {
    env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("/run/user/{}", Uid::current()))
}

pub fn is_gamescope_socket_file(file_name: &str) -> bool {
    file_name.starts_with("gamescope-")
        && !file_name.ends_with(".lock")
        && file_name
            .split('-')
            .next_back()
            .map(|x| x.parse::<u16>().is_ok())
            .unwrap_or_default()
}
