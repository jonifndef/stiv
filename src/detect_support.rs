use std::env;

pub fn is_tmux() -> bool {
    for env_var in ["TERM", "TERMINAL"] {
        if let Ok(env_val) = env::var(env_var) {
            let env_val = env_val.to_ascii_lowercase();
            if env_val.contains("tmux") {
                log::debug!(" -> this terminal seems to be Tmux");

                return true;
            }
        }
    }

    false
}

pub fn get_tmux_nest_count() -> u32 {
    std::env::var("TMUX_NEST_COUNT")
        .map(|s| str::parse(&s).unwrap_or(1))
        .unwrap_or(1)
}

pub fn is_ssh() -> bool {
    for env_var in ["SSH_CLIENT", "SSH_CONNECTION"] {
        if env::var(env_var).is_ok() {
            log::debug!(" -> this seems to be under SSH");

            return true;
        }
    }

    //true // we only support direct stream as of now
    false
}
