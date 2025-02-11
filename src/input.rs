use crate::usage;
use std::{env, process};

pub fn args() -> Vec<String> {
    let args: Vec<_> = env::args().filter(|arg| !arg.is_empty()).collect();

    if args.len() != 3 {
        usage::usage();
        process::exit(1);
    }

    args
}
