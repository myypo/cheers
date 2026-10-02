//! Every name `html!` skips checking must pass the check it would otherwise emit.

#![deny(ambiguous_glob_imports)]

macros::__validation_table_checks!();
