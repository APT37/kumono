use crate::usage::usage;
use std::{env, process::exit};

pub fn args() -> Vec<String> {
    let args: Vec<_> = env::args().filter(|arg| !arg.is_empty()).collect();

    if args.len() != 3 {
        usage();
        exit(1);
    }

    args
}
